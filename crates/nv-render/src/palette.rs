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
        "dusk" => [0x2B_10_55, 0x7B_2C_BF, 0xE0_45_7B, 0xFF_7B_54, 0xFF_B2_6B]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "candy" => [0x72_09_B7, 0xF7_25_85, 0xFF_5D_8F, 0x4C_C9_F0, 0x43_61_EE]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "flamingo" => [0x5C_0A_4E, 0xB5_17_9E, 0xFF_4D_6D, 0xFF_8F_A3, 0xFF_B4_A2]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "orchid" => [0x3C_09_6C, 0x7B_2C_BF, 0xC7_7D_FF, 0xFF_6A_D5, 0xFF_B3_F0]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "sakura" => [0x6B_0F_3A, 0xD8_11_59, 0xFF_5D_8F, 0xFF_97_B7, 0xFF_C4_D6]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "berry" => [0x59_0D_22, 0x80_0F_2F, 0xC9_18_4A, 0xFF_4D_6D, 0xFF_8F_A3]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "midnight" => [0x10_00_2B, 0x3C_18_74, 0x6A_2C_70, 0xB8_3B_5E, 0xF0_8A_5D]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "lava" => [0x3D_0C_11, 0x9D_02_08, 0xE0_1E_37, 0xFF_5E_5B, 0xFF_9E_80]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "electric" => [0x3A_0C_A3, 0x43_61_EE, 0x72_09_B7, 0xF7_25_85, 0xFF_00_6E]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "infrared" => [0x37_06_17, 0x9D_02_08, 0xFF_00_54, 0xFF_20_6E, 0xFF_70_A6]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "coral" => [0x7A_0A_14, 0xC1_12_1F, 0xFF_4D_4D, 0xFF_7F_50, 0xFF_A0_7A]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "ultraviolet" => [0x24_00_46, 0x3C_09_6C, 0x7B_2C_BF, 0xB1_00_E8, 0xE5_00_A4]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "inferno" => [0x00_00_04, 0x42_0A_68, 0x93_26_67, 0xDD_51_3A, 0xFC_A5_0A, 0xFC_FF_A4]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "magma" => [0x00_00_04, 0x3B_0F_70, 0x8C_29_81, 0xDE_49_68, 0xFE_9F_6D, 0xFC_FD_BF]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "sunset" => [0x2D_1B_69, 0x7B_2D_8E, 0xE8_3F_6F, 0xFF_8A_5B, 0xFF_D1_66]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "ocean" => [0x03_04_5E, 0x00_77_B6, 0x00_B4_D8, 0x90_E0_EF, 0xCA_F0_F8]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "aurora" => [0x0B_13_2B, 0x3A_0C_A3, 0x1B_99_8B, 0x2D_E1_C2, 0xC6_F9_1F]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "vaporwave" => [0x1A_0B_3D, 0xB9_67_FF, 0xFF_71_CE, 0x01_CD_FE, 0xFF_FB_96]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "cyberpunk" => [0x0D_02_21, 0x65_0D_89, 0xF7_06_CF, 0x2D_E2_E6, 0xF9_C8_0E]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "fire" => [0x1A_00_00, 0x7F_00_00, 0xE0_3C_00, 0xFF_8C_00, 0xFF_E0_66]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "ice" => [0x0A_11_28, 0x1D_4E_89, 0x5D_A9_E9, 0xB8_E1_FF, 0xFF_FF_FF]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "acid" => [0x12_00_18, 0x5D_00_A3, 0xFF_00_FF, 0x39_FF_14, 0xE6_FF_00]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "gold" => [0x1C_12_06, 0x6B_4A_12, 0xC9_96_2B, 0xF2_D0_6B, 0xFF_F4_CF]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "rose" => [0x1F_0A_1A, 0x6D_1B_4B, 0xC2_30_6F, 0xF4_97_B6, 0xFF_E3_EE]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
        "forest" => [0x0B_1F_14, 0x1E_56_31, 0x4C_9A_2A, 0xA4_DE_02, 0xE8_F5_B5]
            .iter()
            .map(|h| srgb(*h))
            .collect(),
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

#[cfg(test)]
mod named_palettes {
    use super::*;

    #[test]
    fn every_named_palette_is_its_own() {
        let names = [
            "viridis",
            "plasma",
            "turbo",
            "neon",
            "mono",
            "inferno",
            "magma",
            "sunset",
            "ocean",
            "aurora",
            "vaporwave",
            "cyberpunk",
            "fire",
            "ice",
            "acid",
            "gold",
            "rose",
            "forest",
            "dusk",
            "candy",
            "flamingo",
            "orchid",
            "sakura",
            "berry",
            "midnight",
            "lava",
            "electric",
            "infrared",
            "coral",
            "ultraviolet",
        ];
        for (i, a) in names.iter().enumerate() {
            assert_ne!(stops(a), stops("nsku"), "{a} fell back to the default");
            for b in &names[i + 1..] {
                assert_ne!(stops(a), stops(b), "{a} and {b} are the same");
            }
        }
    }
}
