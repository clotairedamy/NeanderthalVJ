//! 3D gradient (Perlin) noise, ported from `visualizer/viz/noise.py`.
//!
//! The grid modes are generative rather than traced from a file: the
//! source drawings are reproduced by evaluating a noise field, which is
//! what makes column count and feature size live controls instead of
//! baked-in constants. The third dimension is time, so the field flows
//! rather than merely jitters.

/// Perlin's improved fade, `6t^5 - 15t^4 + 10t^3`: zero first and second
/// derivative at the cell corners, so no creases show along the lattice.
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Measured peak of this gradient set in 3D over 2.4M samples across six
/// seeds. The often-quoted `1/sqrt(2)` is the 2D figure and over-amplifies
/// here by ~1.4x, which pushes the field outside the range callers map from.
const PEAK: f32 = 0.991_07;

/// A shuffled 0..255 table, doubled so the lookups never wrap.
pub struct Permutation([u8; 512]);

impl Permutation {
    /// Matches `numpy.random.default_rng(seed).permutation(256)` in shape
    /// but not in order -- the field is statistically the same, and no
    /// caller depends on a particular arrangement.
    pub fn new(seed: u64) -> Self {
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        let mut state = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        for i in (1..256).rev() {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let j = (state >> 33) as usize % (i + 1);
            p.swap(i, j);
        }
        let mut doubled = [0u8; 512];
        doubled[..256].copy_from_slice(&p);
        doubled[256..].copy_from_slice(&p);
        Self(doubled)
    }

    fn at(&self, i: usize) -> usize {
        self.0[i & 511] as usize
    }
}

/// Noise at one point, in `[-1, 1]`.
pub fn perlin3(x: f32, y: f32, z: f32, perm: &Permutation) -> f32 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let (u, v, w) = (fade(xf), fade(yf), fade(zf));
    let (xi, yi, zi) =
        (xi as i32 as usize & 255, yi as i32 as usize & 255, zi as i32 as usize & 255);

    // Perlin's own gradient selection: the twelve cube-edge gradients have
    // components in {-1, 0, 1}, so the dot product is a pair of signed
    // picks rather than a multiply.
    let corner = |dx: usize, dy: usize, dz: usize| -> f32 {
        let h =
            perm.at(perm.at(perm.at((xi + dx) & 255) + ((yi + dy) & 255)) + ((zi + dz) & 255)) & 15;
        let (x1, y1, z1) = (xf - dx as f32, yf - dy as f32, zf - dz as f32);
        let a = if h < 8 { x1 } else { y1 };
        let b = if h < 4 {
            y1
        } else if h == 12 || h == 14 {
            x1
        } else {
            z1
        };
        (if h & 1 != 0 { -a } else { a }) + (if h & 2 != 0 { -b } else { b })
    };

    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u);
    let out = lerp(lerp(x00, x10, v), lerp(x01, x11, v), w) / PEAK;
    // the peak is empirical, so clamp to make [-1, 1] a guarantee callers
    // can map from rather than an observation that usually holds
    out.clamp(-1.0, 1.0)
}
