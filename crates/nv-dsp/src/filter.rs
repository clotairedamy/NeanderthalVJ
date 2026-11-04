//! Butterworth design and zero-phase filtering, matching `scipy.signal`.
//!
//! The band splits the pseudo-stems and the beat grid's punch detection
//! rely on `butter(..., output="sos")` followed by `sosfiltfilt`, so both
//! the design and the forward-backward filtering are reproduced here.

use rustfft::num_complex::Complex;

/// One second-order section: `[b0, b1, b2, a0, a1, a2]`, as SciPy stores it.
pub type Section = [f64; 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandType {
    Low,
    High,
    Band,
}

/// `scipy.signal.butter(order, cutoff, btype, fs, output="sos")`.
///
/// `cutoff` is one frequency in Hz for low/high, two for a band.
pub fn butter(order: usize, cutoff: &[f64], btype: BandType, fs: f64) -> Vec<Section> {
    // analog Butterworth prototype: poles evenly spaced on the left half
    // of the unit circle
    let n = order as i32;
    let poles: Vec<Complex<f64>> = (0..n)
        .map(|i| {
            let m = (-n + 1 + 2 * i) as f64;
            -Complex::new(0.0, std::f64::consts::PI * m / (2.0 * n as f64)).exp()
        })
        .collect();

    // pre-warp, so the digital cutoff lands where it was asked for
    let warp = |f: f64| 4.0 * (std::f64::consts::PI * (f / (fs / 2.0)) / 2.0).tan();

    let (zeros, poles, gain) = match btype {
        BandType::Low => {
            let w = warp(cutoff[0]);
            let p: Vec<Complex<f64>> = poles.iter().map(|p| p * w).collect();
            (Vec::new(), p, w.powi(n))
        }
        BandType::High => {
            let w = warp(cutoff[0]);
            // s -> w/s maps the lowpass prototype onto a highpass
            let p: Vec<Complex<f64>> = poles.iter().map(|p| w / p).collect();
            let k: Complex<f64> = poles.iter().map(|p| -p).product();
            (vec![Complex::new(0.0, 0.0); order], p, (1.0 / k).re)
        }
        BandType::Band => {
            let (w1, w2) = (warp(cutoff[0]), warp(cutoff[1]));
            let (bw, w0) = (w2 - w1, (w1 * w2).sqrt());
            let mut p = Vec::with_capacity(2 * order);
            for pk in &poles {
                let half = pk * (bw / 2.0);
                let disc = (half * half - Complex::new(w0 * w0, 0.0)).sqrt();
                p.push(half + disc);
                p.push(half - disc);
            }
            (vec![Complex::new(0.0, 0.0); order], p, bw.powi(n))
        }
    };

    // bilinear transform onto the unit circle
    let fs2 = 4.0;
    let z_d: Vec<Complex<f64>> = zeros.iter().map(|z| (fs2 + z) / (fs2 - z)).collect();
    let p_d: Vec<Complex<f64>> = poles.iter().map(|p| (fs2 + p) / (fs2 - p)).collect();
    let num: Complex<f64> = zeros.iter().map(|z| fs2 - z).product();
    let den: Complex<f64> = poles.iter().map(|p| fs2 - p).product();
    let k_d = gain * (num / den).re;
    // the transform sends the remaining degrees to the Nyquist point
    let mut z_d = z_d;
    z_d.resize(p_d.len(), Complex::new(-1.0, 0.0));

    zpk_to_sos(&z_d, &p_d, k_d)
}

/// Pair each conjugate pole pair with a zero pair, closest-to-the-unit-
/// circle last, as `scipy.signal.zpk2sos` orders them.
fn zpk_to_sos(zeros: &[Complex<f64>], poles: &[Complex<f64>], gain: f64) -> Vec<Section> {
    let mut z = sort_conjugates(zeros);
    let mut p = sort_conjugates(poles);
    // SciPy emits the section nearest the unit circle last; its ordering
    // puts the *farthest* pole pair first
    p.sort_by(|a, b| a.norm().total_cmp(&b.norm()));
    z.sort_by(|a, b| a.norm().total_cmp(&b.norm()));

    let n_sections = p.len().div_ceil(2);
    let mut out = Vec::with_capacity(n_sections);
    for s in 0..n_sections {
        let pp = &p[2 * s..(2 * s + 2).min(p.len())];
        let zz = if 2 * s < z.len() { &z[2 * s..(2 * s + 2).min(z.len())] } else { &[][..] };
        let (b1, b2) = quadratic_from_roots(zz);
        let (a1, a2) = quadratic_from_roots(pp);
        out.push([1.0, b1, b2, 1.0, a1, a2]);
    }
    // the overall gain rides on the first section
    for c in out[0].iter_mut().take(3) {
        *c *= gain;
    }
    out
}

/// Order roots so each complex pair sits next to its conjugate.
fn sort_conjugates(roots: &[Complex<f64>]) -> Vec<Complex<f64>> {
    let mut real: Vec<Complex<f64>> =
        roots.iter().filter(|r| r.im.abs() < 1e-12).copied().collect();
    let mut pos: Vec<Complex<f64>> = roots.iter().filter(|r| r.im > 1e-12).copied().collect();
    pos.sort_by(|a, b| a.norm().total_cmp(&b.norm()));
    let mut out = Vec::with_capacity(roots.len());
    for r in pos {
        out.push(r);
        out.push(r.conj());
    }
    real.sort_by(|a, b| a.re.total_cmp(&b.re));
    out.extend(real);
    out
}

/// `(c1, c2)` of `z^2 + c1 z + c2` from up to two roots.
fn quadratic_from_roots(roots: &[Complex<f64>]) -> (f64, f64) {
    match roots.len() {
        0 => (0.0, 0.0),
        1 => (-roots[0].re, 0.0),
        _ => {
            let sum = roots[0] + roots[1];
            let prod = roots[0] * roots[1];
            (-sum.re, prod.re)
        }
    }
}

/// Steady-state initial conditions, as `scipy.signal.sosfilt_zi`.
fn sosfilt_zi(sos: &[Section]) -> Vec<[f64; 2]> {
    let mut out = Vec::with_capacity(sos.len());
    let mut scale = 1.0;
    for s in sos {
        let (b0, b1, b2) = (s[0], s[1], s[2]);
        let (a1, a2) = (s[4], s[5]);
        // solve (I - A^T) zi = B for the second-order section
        let (m00, m01, m10, m11) = (1.0 + a1, -1.0, a2, 1.0);
        let (r0, r1) = (b1 - a1 * b0, b2 - a2 * b0);
        let det = m00 * m11 - m01 * m10;
        let z0 = (r0 * m11 - m01 * r1) / det;
        let z1 = (m00 * r1 - r0 * m10) / det;
        out.push([scale * z0, scale * z1]);
        // the next section starts from this one's DC gain
        scale *= (b0 + b1 + b2) / (1.0 + a1 + a2);
    }
    out
}

/// One forward pass of a cascade, seeded with `zi` scaled by `x[0]`.
fn sosfilt(sos: &[Section], x: &[f64], zi: &[[f64; 2]]) -> Vec<f64> {
    let mut state: Vec<[f64; 2]> = zi.to_vec();
    let mut out = Vec::with_capacity(x.len());
    for &sample in x {
        let mut v = sample;
        for (s, st) in sos.iter().zip(state.iter_mut()) {
            let y = s[0] * v + st[0];
            st[0] = s[1] * v - s[4] * y + st[1];
            st[1] = s[2] * v - s[5] * y;
            v = y;
        }
        out.push(v);
    }
    out
}

/// `scipy.signal.sosfiltfilt`: forward then backward, so the result has no
/// phase shift at all -- which is what makes subtracting one band from
/// another meaningful.
pub fn sosfiltfilt(sos: &[Section], x: &[f64]) -> Vec<f64> {
    let n = x.len();
    let padlen = 3 * (2 * sos.len() + 1);
    if n <= padlen {
        return x.to_vec();
    }
    // odd extension: mirror through the endpoint, so the padding carries
    // the signal's slope rather than a step
    let mut ext = Vec::with_capacity(n + 2 * padlen);
    for i in (1..=padlen).rev() {
        ext.push(2.0 * x[0] - x[i]);
    }
    ext.extend_from_slice(x);
    for i in 1..=padlen {
        ext.push(2.0 * x[n - 1] - x[n - 1 - i]);
    }

    let zi = sosfilt_zi(sos);
    let scaled: Vec<[f64; 2]> = zi.iter().map(|z| [z[0] * ext[0], z[1] * ext[0]]).collect();
    let fwd = sosfilt(sos, &ext, &scaled);

    let mut rev: Vec<f64> = fwd.into_iter().rev().collect();
    let scaled: Vec<[f64; 2]> = zi.iter().map(|z| [z[0] * rev[0], z[1] * rev[0]]).collect();
    rev = sosfilt(sos, &rev, &scaled);
    rev.reverse();
    rev[padlen..padlen + n].to_vec()
}

/// `scipy.ndimage.median_filter(x, size, mode="reflect")` over a 2-D array
/// stored as rows, with a strictly horizontal or vertical window.
///
/// HPSS only ever asks for those two, and they are what separates a
/// sustained note (smeared across time) from a transient (across frequency).
pub fn median_filter_1d(rows: &[Vec<f32>], size: usize, horizontal: bool) -> Vec<Vec<f32>> {
    let (h, w) = (rows.len(), rows.first().map_or(0, |r| r.len()));
    let half = size / 2;
    // `reflect` mirrors about the edge *sample*: ... c b a | a b c ...
    let clamp = |i: isize, n: usize| -> usize {
        let n = n as isize;
        let mut i = i;
        loop {
            if i < 0 {
                i = -i - 1;
            } else if i >= n {
                i = 2 * n - i - 1;
            } else {
                return i as usize;
            }
        }
    };
    let mut out = vec![vec![0.0f32; w]; h];
    let mut buf = vec![0.0f32; size];
    for y in 0..h {
        for x in 0..w {
            for (k, b) in buf.iter_mut().enumerate() {
                let off = k as isize - half as isize;
                *b = if horizontal {
                    rows[y][clamp(x as isize + off, w)]
                } else {
                    rows[clamp(y as isize + off, h)][x]
                };
            }
            buf.sort_by(|a, b| a.total_cmp(b));
            out[y][x] = if size % 2 == 1 { buf[half] } else { 0.5 * (buf[half - 1] + buf[half]) };
        }
    }
    out
}
