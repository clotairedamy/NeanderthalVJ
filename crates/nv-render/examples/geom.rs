//! Build the bust model and print its shape, to compare against the
//! Python implementation it is ported from.

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| "assets".into());
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(200_000);
    let glb = format!("{root}/LeePerrySmith.glb");
    let nmap = format!("{root}/Infinite-Level_02_Tangent_SmoothUV.jpg");

    let t0 = std::time::Instant::now();
    let m =
        match nv_render::bust::BustModel::build(&glb, Some(std::path::Path::new(&nmap)), n, 3.6, 7)
        {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };
    let took = t0.elapsed();

    let xs: Vec<f32> = m.particles.iter().map(|p| p.pos[0]).collect();
    let ys: Vec<f32> = m.particles.iter().map(|p| p.pos[1]).collect();
    let zs: Vec<f32> = m.particles.iter().map(|p| p.pos[2]).collect();
    let mm = |v: &[f32]| {
        (
            v.iter().copied().fold(f32::INFINITY, f32::min),
            v.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        )
    };
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let zones: Vec<f32> = m.particles.iter().map(|p| p.zone).collect();
    let fades: Vec<f32> = m.particles.iter().map(|p| p.fade).collect();

    println!(
        "built {} particles, {} shell verts, {} indices in {took:?}",
        m.particles.len(),
        m.shell.len(),
        m.indices.len()
    );
    println!("  x range {:?}  mean {:+.4}", mm(&xs), mean(&xs));
    println!("  y range {:?}", mm(&ys));
    println!("  z range {:?}  span {:.4}", mm(&zs), mm(&zs).1 - mm(&zs).0);
    println!("  zone    {:?}", mm(&zones));
    println!("  fade    {:?}", mm(&fades));
    println!("  pivot   [{:+.4}, {:+.4}, {:+.4}]", m.pivot.x, m.pivot.y, m.pivot.z);
    let shell_z = mm(&m.shell.iter().map(|v| v.pos[2]).collect::<Vec<_>>());
    println!("  shell z {:?}", shell_z);
    let finite = m.particles.iter().all(|p| p.pos.iter().chain(&p.nrm).all(|v| v.is_finite()));
    println!("  all finite: {finite}");
    // how much the normals actually vary: the normal map should scatter
    // them well beyond the smooth mesh normals
    let nx: Vec<f32> = m.particles.iter().map(|p| p.nrm[0]).collect();
    let ny: Vec<f32> = m.particles.iter().map(|p| p.nrm[1]).collect();
    let nz: Vec<f32> = m.particles.iter().map(|p| p.nrm[2]).collect();
    let sd = |v: &[f32]| {
        let mu = v.iter().sum::<f32>() / v.len() as f32;
        (v.iter().map(|x| (x - mu) * (x - mu)).sum::<f32>() / v.len() as f32).sqrt()
    };
    println!("  normal sd  x {:.4} y {:.4} z {:.4}", sd(&nx), sd(&ny), sd(&nz));
    let len_ok = m
        .particles
        .iter()
        .filter(|p| {
            let n = glam::Vec3::from(p.nrm).length();
            (n - 1.0).abs() < 1e-3
        })
        .count();
    println!("  unit normals: {}/{}", len_ok, m.particles.len());
    let c = m.particles.iter().fold(glam::Vec3::ZERO, |a, p| a + glam::Vec3::from(p.pos))
        / m.particles.len() as f32;
    let outward = m
        .particles
        .iter()
        .filter(|p| (glam::Vec3::from(p.pos) - c).dot(glam::Vec3::from(p.nrm)) > 0.0)
        .count();
    println!(
        "  outward-pointing normals: {:.1}%",
        100.0 * outward as f32 / m.particles.len() as f32
    );
}
