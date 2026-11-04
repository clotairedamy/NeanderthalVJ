//! The scanned head as a point cloud (`visualizer/viz/bust.py`).
//!
//! The subject is "Infinite", a head scan by Lee Perry-Smith (CC BY 3.0,
//! via the three.js examples): head, neck and shoulders of a real person,
//! sampled into fine grayscale particles whose normals carry the scan's
//! normal map, so pores and the lips' edges catch a hard side light while
//! the far side falls into black.
//!
//! Everything here is built once. Motion is uniforms only, which is what
//! makes this mode the natural first one to port: there is no per-frame
//! CPU work to get wrong.

use std::path::Path;

use glam::{Mat3, Vec2, Vec3};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// glTF is Y-up facing +Z; the visualizer is Z-up facing -Y, the camera side.
fn to_world(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.z, v.y)
}

/// One particle, as the vertex shader reads it.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pub pos: [f32; 3],
    /// 0 at the shoulders, ~0.38 at the neck, 1 at the crown. The slice
    /// bands and the per-zone pushes all live along this.
    pub zone: f32,
    pub nrm: [f32; 3],
    /// Spot-lit falloff: the shoulders sink into the dark.
    pub fade: f32,
    /// Per-particle randomness, the seed for every hash in the shader.
    pub rnd: f32,
    pub _pad: [f32; 3],
}

/// The scan's surface as a solid shell, drawn in the background colour so
/// the depth test hides the mouth's interior and the far side of the head,
/// which otherwise glint through the gaps between particles.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShellVertex {
    pub pos: [f32; 3],
    pub zone: f32,
    pub nrm: [f32; 3],
    pub _pad: f32,
}

pub struct BustModel {
    pub particles: Vec<Particle>,
    pub shell: Vec<ShellVertex>,
    pub indices: Vec<u32>,
    /// The neck, where the head turns and tilts.
    pub pivot: Vec3,
    pub zmin: f32,
    pub zmax: f32,
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bust model: {}", self.0)
    }
}

impl std::error::Error for LoadError {}

fn unit(v: Vec3) -> Vec3 {
    let n = v.length();
    if n > 1e-12 {
        v / n
    } else {
        v
    }
}

/// Snap loose surface patches onto the main mesh.
///
/// The scan's lower cheeks are separate patches whose borders float ~0.06
/// units off the hole they cover, which shows as a flap with a hard edge
/// along the jaw. Each patch border vertex is moved onto the nearest
/// main-surface vertex, and the patch interior follows by inverse-distance
/// blending of those corrections, so the jaw becomes one surface.
fn glue_patches(p: &mut [Vec3], faces: &[[u32; 3]], max_gap: f32) {
    let n = p.len();
    // connected components over the edge graph, by union-find
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let union = |parent: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (find(parent, a), find(parent, b));
        if ra != rb {
            parent[ra] = rb;
        }
    };
    for f in faces {
        union(&mut parent, f[0] as usize, f[1] as usize);
        union(&mut parent, f[1] as usize, f[2] as usize);
    }
    let label: Vec<usize> = (0..n).map(|i| find(&mut parent, i)).collect();

    let mut counts = std::collections::HashMap::<usize, usize>::new();
    for l in &label {
        *counts.entry(*l).or_default() += 1;
    }
    if counts.len() < 2 {
        return;
    }
    let main = *counts.iter().max_by_key(|(_, c)| **c).map(|(l, _)| l).unwrap();

    // border vertices: those on an edge used by exactly one face
    let mut edge_uses = std::collections::HashMap::<(u32, u32), u32>::new();
    for f in faces {
        for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            *edge_uses.entry(key).or_default() += 1;
        }
    }
    let mut border = vec![false; n];
    for ((a, b), used) in &edge_uses {
        if *used == 1 {
            border[*a as usize] = true;
            border[*b as usize] = true;
        }
    }

    let main_idx: Vec<usize> = (0..n).filter(|i| label[*i] == main).collect();
    let islands: Vec<usize> = counts.keys().copied().filter(|l| *l != main).collect();

    for isl in islands {
        let vi: Vec<usize> = (0..n).filter(|i| label[*i] == isl).collect();
        let bv: Vec<usize> = vi.iter().copied().filter(|i| border[*i]).collect();
        if bv.len() < 3 {
            continue;
        }
        // nearest main-surface vertex for each border vertex; 9k vertices
        // makes a brute-force search cheaper than building an index
        let mut moved: Vec<(usize, Vec3)> = Vec::new();
        let mut gaps: Vec<f32> = Vec::new();
        for &b in &bv {
            let mut best = (f32::INFINITY, 0usize);
            for &m in &main_idx {
                let d = p[b].distance_squared(p[m]);
                if d < best.0 {
                    best = (d, m);
                }
            }
            let d = best.0.sqrt();
            if d < max_gap {
                moved.push((b, p[best.1] - p[b]));
                gaps.push(d);
            }
        }
        if moved.len() < 3 {
            continue;
        }
        gaps.sort_by(f32::total_cmp);
        if gaps[gaps.len() / 2] < 1e-4 {
            continue; // already sealed (a UV seam)
        }
        // the interior follows its border by inverse-distance weighting
        let originals: Vec<Vec3> = vi.iter().map(|i| p[*i]).collect();
        let anchors: Vec<Vec3> = moved.iter().map(|(b, _)| p[*b]).collect();
        for (k, &i) in vi.iter().enumerate() {
            let mut acc = Vec3::ZERO;
            let mut wsum = 0.0;
            for (j, (_, shift)) in moved.iter().enumerate() {
                let d = originals[k].distance(anchors[j]).max(1e-6);
                let w = 1.0 / (d * d);
                acc += *shift * w;
                wsum += w;
            }
            if wsum > 0.0 {
                p[i] += acc / wsum;
            }
        }
    }
}

/// Per-vertex tangents from the UV gradients, for the tangent-space normals.
fn vertex_tangents(p: &[Vec3], uv: &[Vec2], faces: &[[u32; 3]]) -> Vec<Vec3> {
    let mut t = vec![Vec3::ZERO; p.len()];
    for f in faces {
        let (i0, i1, i2) = (f[0] as usize, f[1] as usize, f[2] as usize);
        let (e1, e2) = (p[i1] - p[i0], p[i2] - p[i0]);
        let (d1, d2) = (uv[i1] - uv[i0], uv[i2] - uv[i0]);
        let mut r = d1.x * d2.y - d2.x * d1.y;
        if r.abs() < 1e-12 {
            r = 1e-12;
        }
        let tan = (e1 * d2.y - e2 * d1.y) / r;
        t[i0] += tan;
        t[i1] += tan;
        t[i2] += tan;
    }
    t
}

impl BustModel {
    /// Build the model from the scan and its normal map.
    ///
    /// `span` is how many world units the bust covers from shoulders to
    /// crown; `n_points` is how finely the surface is sampled.
    pub fn build(
        glb: impl AsRef<Path>,
        normal_map: Option<&Path>,
        n_points: usize,
        span: f32,
        seed: u64,
    ) -> Result<Self, LoadError> {
        let (doc, buffers, _) = gltf::import(glb.as_ref()).map_err(|e| LoadError(e.to_string()))?;
        let mesh = doc.meshes().next().ok_or_else(|| LoadError("no mesh".into()))?;
        let prim = mesh.primitives().next().ok_or_else(|| LoadError("no primitive".into()))?;
        let reader = prim.reader(|b| buffers.get(b.index()).map(|d| &d.0[..]));

        let pos: Vec<Vec3> = reader
            .read_positions()
            .ok_or_else(|| LoadError("no POSITION".into()))?
            .map(Vec3::from)
            .collect();
        let nrm: Vec<Vec3> = reader
            .read_normals()
            .ok_or_else(|| LoadError("no NORMAL".into()))?
            .map(Vec3::from)
            .collect();
        let uv: Vec<Vec2> = reader
            .read_tex_coords(0)
            .ok_or_else(|| LoadError("no TEXCOORD_0".into()))?
            .into_f32()
            .map(Vec2::from)
            .collect();
        let idx: Vec<u32> = reader
            .read_indices()
            .ok_or_else(|| LoadError("no indices".into()))?
            .into_u32()
            .collect();
        let faces: Vec<[u32; 3]> = idx.as_chunks::<3>().0.to_vec();

        let mut pos = pos;
        glue_patches(&mut pos, &faces, 0.3);
        let tangents = vertex_tangents(&pos, &uv, &faces);

        // sample the surface, each triangle in proportion to its area
        let areas: Vec<f32> = faces
            .iter()
            .map(|f| {
                let (a, b, c) = (pos[f[0] as usize], pos[f[1] as usize], pos[f[2] as usize]);
                0.5 * (b - a).cross(c - a).length()
            })
            .collect();
        let total: f32 = areas.iter().sum();
        let mut cdf = Vec::with_capacity(areas.len());
        let mut acc = 0.0;
        for a in &areas {
            acc += a / total;
            cdf.push(acc);
        }

        let normal_tex = normal_map.and_then(|p| image::open(p).ok()).map(|i| i.to_rgb8());
        let mut rng = Pcg64Mcg::seed_from_u64(seed);

        let mut raw_pos = Vec::with_capacity(n_points);
        let mut raw_nrm = Vec::with_capacity(n_points);
        for _ in 0..n_points {
            let r: f32 = rng.gen();
            let tri = cdf.partition_point(|c| *c < r).min(faces.len() - 1);
            let f = faces[tri];
            // a uniform point in the triangle
            let s1 = rng.gen::<f32>().sqrt();
            let r2: f32 = rng.gen();
            let bary = Vec3::new(1.0 - s1, s1 * (1.0 - r2), s1 * r2);
            let (i0, i1, i2) = (f[0] as usize, f[1] as usize, f[2] as usize);

            let p = pos[i0] * bary.x + pos[i1] * bary.y + pos[i2] * bary.z;
            let mut n = unit(nrm[i0] * bary.x + nrm[i1] * bary.y + nrm[i2] * bary.z);
            let t = tangents[i0] * bary.x + tangents[i1] * bary.y + tangents[i2] * bary.z;
            let w = uv[i0] * bary.x + uv[i1] * bary.y + uv[i2] * bary.z;

            // the scan's normal map: skin detail the 9k-vertex mesh cannot hold
            if let Some(tex) = &normal_tex {
                let tan = unit(t - n * t.dot(n));
                let bit = n.cross(tan);
                let (tw, th) = (tex.width(), tex.height());
                let x = ((w.x.rem_euclid(1.0)) * tw as f32) as u32;
                let y = ((w.y.rem_euclid(1.0)) * th as f32) as u32;
                let px = tex.get_pixel(x.min(tw - 1), y.min(th - 1));
                let d = Vec3::new(
                    px[0] as f32 / 255.0 * 2.0 - 1.0,
                    px[1] as f32 / 255.0 * 2.0 - 1.0,
                    px[2] as f32 / 255.0 * 2.0 - 1.0,
                );
                n = unit(tan * d.x + bit * d.y + n * d.z);
            }
            raw_pos.push(to_world(p));
            raw_nrm.push(to_world(n));
        }

        // centre and scale so the bust spans `span` from shoulders to crown
        let (mut zmin, mut zmax) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in &raw_pos {
            zmin = zmin.min(p.z);
            zmax = zmax.max(p.z);
        }
        let s = span / (zmax - zmin);
        let centre = Vec3::new(0.0, 0.0, 0.5 * (zmin + zmax));
        let scale = |p: Vec3| (p - centre) * s;

        let scaled: Vec<Vec3> = raw_pos.iter().map(|p| scale(*p)).collect();
        let (mut zmin, mut zmax) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in &scaled {
            zmin = zmin.min(p.z);
            zmax = zmax.max(p.z);
        }
        let zone_of = |z: f32| (z - zmin) / (zmax - zmin);

        let mut rnd = Pcg64Mcg::seed_from_u64(seed ^ 0x9E37_79B9);
        let particles: Vec<Particle> = scaled
            .iter()
            .zip(&raw_nrm)
            .map(|(p, n)| {
                let zone = zone_of(p.z);
                Particle {
                    pos: p.to_array(),
                    zone,
                    nrm: n.to_array(),
                    // spot-lit falloff: the shoulders sink into the dark
                    fade: (zone * 2.4 - 0.25).clamp(0.0, 1.0),
                    rnd: rnd.gen(),
                    _pad: [0.0; 3],
                }
            })
            .collect();

        let shell: Vec<ShellVertex> = pos
            .iter()
            .zip(&nrm)
            .map(|(p, n)| {
                let wp = scale(to_world(*p));
                ShellVertex {
                    pos: wp.to_array(),
                    zone: zone_of(wp.z),
                    nrm: to_world(*n).to_array(),
                    _pad: 0.0,
                }
            })
            .collect();

        // the neck pivot: where the head turns on the shoulders
        let neck: Vec<&Particle> =
            particles.iter().filter(|p| p.zone > 0.30 && p.zone < 0.42).collect();
        let pivot_y = if neck.is_empty() {
            0.0
        } else {
            neck.iter().map(|p| p.pos[1]).sum::<f32>() / neck.len() as f32
        };
        let pivot = Vec3::new(0.0, pivot_y, zmin + 0.38 * (zmax - zmin));

        Ok(Self { particles, shell, indices: idx, pivot, zmin, zmax })
    }
}

/// The rotation the shader applies; kept here so the CPU can agree with it.
pub fn head_rotation(yaw: f32, pitch: f32) -> Mat3 {
    Mat3::from_rotation_x(-pitch) * Mat3::from_rotation_z(yaw)
}
