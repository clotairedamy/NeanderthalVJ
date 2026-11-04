//! Colour palettes, after `visualizer/color/palette.py`.
//!
//! The head is grayscale by default -- it is a spot-lit photograph, and the
//! Python leaves it that way deliberately ("the Depth Point Cloud keeps its
//! own look"). A palette tints it when one is chosen, mixed in rather than
//! replacing the light, so the modelling survives.

/// The NSKU website palette: each chapter pairs a deep hue with a bright one.
const NSKU: [[f32; 3]; 2] = [[0.247, 0.42, 0.071], [0.784, 0.961, 0.376]];

fn srgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

/// Control points of each named palette, low to high.
pub fn stops(name: &str) -> Vec<[f32; 3]> {
    match name {
        "viridis" => [0x44_01_54, 0x3B_52_8B, 0x21_90_8C, 0x5E_C9_62, 0xFD_E7_25]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "plasma" => [0x0D_08_87, 0x7E_03_A8, 0xCC_47_6B, 0xF8_9C_3F, 0xF0_F7_21]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "turbo" => [0x30_12_3B, 0x28_82_DB, 0x1A_E4_B6, 0xD2_E2_1B, 0xD2_32_05]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "neon" => {
            [0x00_F5_FF, 0x7A_2B_FF, 0xFF_2B_8A, 0xFF_E5_3B].iter().map(|h| srgb(*h)).collect()
        }
        "mono" => vec![[0.12, 0.12, 0.13], [1.0, 1.0, 1.0]],
        // "nsku" and anything unknown
        _ => NSKU.to_vec(),
    }
}

/// The palette's colour at `t` in 0..1.
pub fn sample(name: &str, t: f32) -> [f32; 3] {
    sample_stops(&stops(name), t)
}

/// The colour at `t` in 0..1 along any list of stops, low to high.
pub fn sample_stops(s: &[[f32; 3]], t: f32) -> [f32; 3] {
    match s.len() {
        0 => return [1.0; 3],
        1 => return s[0],
        _ => {}
    }
    let t = t.clamp(0.0, 1.0) * (s.len() - 1) as f32;
    let i = (t.floor() as usize).min(s.len() - 2);
    let f = t - i as f32;
    let (a, b) = (s[i], s[i + 1]);
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

/// The colour a band drives, as `band_color` does: the spectrum laid
/// across the palette, low to high.
pub fn band_color(name: &str, band: usize, n_bands: usize) -> [f32; 3] {
    sample(name, band as f32 / (n_bands.max(2) - 1) as f32)
}

/// `#rrggbb` as a colour, if it is one.
pub fn parse_hex(hex: &str) -> Option<[f32; 3]> {
    let h = hex.trim().trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).ok().filter(|_| h.len() == 6)?;
    Some(srgb(v))
}

/// A colour as `#rrggbb`.
pub fn to_hex(c: [f32; 3]) -> String {
    let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2]))
}

fn luminance(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The `k` colours a picture is mostly made of, darkest first: k-means
/// over a thumbnail, seeded across the brightness range so a dark photo
/// still yields its highlights.
pub fn from_pixels(pixels: &[[f32; 3]], k: usize) -> Vec<[f32; 3]> {
    if pixels.is_empty() || k == 0 {
        return Vec::new();
    }
    let mut sorted: Vec<[f32; 3]> = pixels.to_vec();
    sorted.sort_by(|a, b| luminance(*a).total_cmp(&luminance(*b)));
    let mut centres: Vec<[f32; 3]> =
        (0..k).map(|i| sorted[(i * 2 + 1) * sorted.len() / (2 * k)]).collect();
    let mut sums = vec![([0.0f32; 3], 0usize); k];
    for _ in 0..12 {
        sums.iter_mut().for_each(|s| *s = ([0.0; 3], 0));
        for p in pixels {
            let nearest = (0..k)
                .min_by(|a, b| {
                    let d = |c: [f32; 3]| {
                        (c[0] - p[0]).powi(2) + (c[1] - p[1]).powi(2) + (c[2] - p[2]).powi(2)
                    };
                    d(centres[*a]).total_cmp(&d(centres[*b]))
                })
                .unwrap_or(0);
            let s = &mut sums[nearest];
            s.0 = [s.0[0] + p[0], s.0[1] + p[1], s.0[2] + p[2]];
            s.1 += 1;
        }
        for (c, (sum, n)) in centres.iter_mut().zip(&sums) {
            if *n > 0 {
                *c = [sum[0] / *n as f32, sum[1] / *n as f32, sum[2] / *n as f32];
            }
        }
    }
    centres.sort_by(|a, b| luminance(*a).total_cmp(&luminance(*b)));
    centres
}

/// The five colours of the picture at `path`, darkest first.
pub fn from_image(path: &std::path::Path) -> Result<Vec<[f32; 3]>, String> {
    let img = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let thumb = img.thumbnail(64, 64).to_rgb8();
    let pixels: Vec<[f32; 3]> = thumb
        .pixels()
        .map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0])
        .collect();
    Ok(from_pixels(&pixels, 5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_of_two_colours_gives_back_those_colours() {
        let red = [0.9, 0.1, 0.1];
        let navy = [0.05, 0.05, 0.3];
        let pixels: Vec<[f32; 3]> = (0..400).map(|i| if i % 3 == 0 { red } else { navy }).collect();
        let found = from_pixels(&pixels, 2);
        assert_eq!(found.len(), 2);
        let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02);
        // darkest first
        assert!(close(found[0], navy) && close(found[1], red), "{found:?}");
    }

    #[test]
    fn colours_round_trip_through_hex() {
        assert_eq!(parse_hex("#ff3d7f").map(to_hex).as_deref(), Some("#ff3d7f"));
        assert_eq!(parse_hex("nope"), None);
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn sampling_any_stops_interpolates_between_them() {
        let s = [[0.0; 3], [1.0; 3]];
        assert_eq!(sample_stops(&s, 0.5), [0.5; 3]);
        assert_eq!(sample_stops(&[], 0.5), [1.0; 3]);
        assert_eq!(sample_stops(&[[0.2; 3]], 0.9), [0.2; 3]);
    }
}
