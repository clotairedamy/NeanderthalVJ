//! Numerical parity with librosa, for the functions nv-dsp replaces.
//!
//! `tools/capture_dsp.py` runs a deterministic 12-second groove through
//! librosa and writes its output; this replays the same samples and
//! asserts agreement. Continuous results are compared to float32 rounding;
//! discrete ones (tempo, beat frames, onset frames) must match exactly,
//! since a one-frame shift is a real behavioural change.

use std::fs;
use std::sync::OnceLock;

use serde_json::Value;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

pub struct Fixture {
    pub doc: Value,
    pub y: Vec<f32>,
}

pub fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let hint =
            "missing fixtures -- they are checked in under tests/fixtures; restore them with git";
        let doc: Value = serde_json::from_str(
            &fs::read_to_string(format!("{FIXTURES}/expected.json")).expect(hint),
        )
        .unwrap();
        let y = fs::read(format!("{FIXTURES}/input.f32"))
            .expect(hint)
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        Fixture { doc, y }
    })
}

fn floats(v: &Value) -> Vec<f32> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap() as f32).collect()
}

/// Worst absolute and relative deviation across a pair of series.
fn worst(got: &[f32], want: &[f32]) -> (f64, f64) {
    assert_eq!(got.len(), want.len(), "length mismatch");
    let mut abs = 0.0f64;
    let mut rel = 0.0f64;
    for (g, w) in got.iter().zip(want) {
        let d = (*g as f64 - *w as f64).abs();
        abs = abs.max(d);
        let scale = (*w as f64).abs().max(1e-6);
        rel = rel.max(d / scale);
    }
    (abs, rel)
}

#[test]
fn stft_matches_librosa() {
    let f = fixture();
    let (n_fft, hop) =
        (f.doc["n_fft"].as_u64().unwrap() as usize, f.doc["hop"].as_u64().unwrap() as usize);
    let stft = nv_dsp::Stft::new(n_fft, hop);
    let s = stft.forward(&f.y);

    assert_eq!(s.n_frames, f.doc["n_frames"].as_u64().unwrap() as usize, "frame count");
    assert_eq!(s.n_bins, f.doc["n_bins"].as_u64().unwrap() as usize, "bin count");

    let mag = s.magnitude();
    let idx: Vec<usize> = f.doc["mag_frame_idx"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    // Judged against each frame's own peak: a spectrogram bin 70 dB below
    // the loudest one carries no information, and holding it to a relative
    // tolerance would only be measuring float32 epsilon.
    let mut worst_rel_peak = 0.0f64;
    for (k, &fr) in idx.iter().enumerate() {
        let want = floats(&f.doc["mag_frames"][k]);
        let peak = want.iter().fold(0.0f32, |a, b| a.max(*b)) as f64;
        let (a, _) = worst(&mag[fr], &want);
        worst_rel_peak = worst_rel_peak.max(a / peak);
    }
    println!("stft magnitude: worst error vs frame peak {worst_rel_peak:.3e}");
    assert!(worst_rel_peak < 1e-6, "stft differs from librosa: {worst_rel_peak:.3e} of peak");
}

#[test]
fn istft_roundtrip_matches_librosa() {
    let f = fixture();
    let (n_fft, hop) =
        (f.doc["n_fft"].as_u64().unwrap() as usize, f.doc["hop"].as_u64().unwrap() as usize);
    let stft = nv_dsp::Stft::new(n_fft, hop);
    let s = stft.forward(&f.y);
    let recon = stft.inverse(&s, Some(f.y.len()));

    assert_eq!(recon.len(), f.doc["n_samples"].as_u64().unwrap() as usize, "length");
    let (a1, r1) = worst(&recon[..512], &floats(&f.doc["recon_head"]));
    let mid = recon.len() / 2;
    let (a2, r2) = worst(&recon[mid..mid + 512], &floats(&f.doc["recon_mid"]));
    let rms =
        (recon.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>() / recon.len() as f64).sqrt();
    let want_rms = f.doc["recon_rms"].as_f64().unwrap();
    println!(
        "istft: head abs {a1:.3e} rel {r1:.3e} | mid abs {a2:.3e} rel {r2:.3e} | rms {rms:.6} vs {want_rms:.6}"
    );
    assert!(a1.max(a2) < 1e-5, "istft differs from librosa");
    assert!((rms - want_rms).abs() < 1e-6, "istft rms differs");
}

#[test]
fn mel_filterbank_matches_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let n_fft = f.doc["n_fft"].as_u64().unwrap() as usize;
    let basis = nv_dsp::mel::mel_filterbank(sr, n_fft, nv_dsp::onset::N_MELS);

    let mut abs = 0.0f64;
    for (k, &row) in [0usize, 1, 40, 127].iter().enumerate() {
        let (a, _) = worst(&basis[row], &floats(&f.doc["mel_basis_rows"][k]));
        abs = abs.max(a);
    }
    println!("mel basis: worst abs {abs:.3e}");
    assert!(abs < 1e-9, "mel filterbank differs from librosa: {abs:.3e}");
}

#[test]
fn melspectrogram_and_db_match_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let n_fft = f.doc["n_fft"].as_u64().unwrap() as usize;
    let hop = f.doc["hop"].as_u64().unwrap() as usize;

    let stft = nv_dsp::Stft::new(n_fft, hop);
    let s = stft.forward(&f.y);
    let basis = nv_dsp::mel::mel_filterbank(sr, n_fft, nv_dsp::onset::N_MELS);
    let mut mel = nv_dsp::mel::apply_filterbank(&basis, &s.power(2.0));

    let idx = [0usize, 40, s.n_frames / 2];
    let mut rel_peak = 0.0f64;
    for (k, &fr) in idx.iter().enumerate() {
        let want = floats(&f.doc["melspec_frames"][k]);
        let peak = want.iter().fold(0.0f32, |a, b| a.max(*b)) as f64;
        let (a, _) = worst(&mel[fr], &want);
        rel_peak = rel_peak.max(a / peak);
    }
    println!("melspectrogram: worst error vs frame peak {rel_peak:.3e}");
    assert!(rel_peak < 1e-6, "melspectrogram differs: {rel_peak:.3e} of peak");

    // dB is where small power differences get compressed, so it is held to
    // an absolute tolerance in decibels
    nv_dsp::mel::power_to_db(&mut mel);
    let mut abs = 0.0f64;
    for (k, &fr) in idx.iter().enumerate() {
        let (a, _) = worst(&mel[fr], &floats(&f.doc["db_frames"][k]));
        abs = abs.max(a);
    }
    println!("power_to_db: worst abs {abs:.3e} dB");
    assert!(abs < 1e-3, "power_to_db differs from librosa: {abs:.3e} dB");
}

#[test]
fn onset_strength_matches_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let n_fft = f.doc["n_fft"].as_u64().unwrap() as usize;
    let hop = f.doc["hop"].as_u64().unwrap() as usize;

    for (name, agg) in
        [("env_mean", nv_dsp::Aggregate::Mean), ("env_median", nv_dsp::Aggregate::Median)]
    {
        let got = nv_dsp::onset_strength(&f.y, sr, n_fft, hop, agg);
        let want = floats(&f.doc[name]);
        assert_eq!(got.len(), want.len(), "{name}: length");
        let peak = want.iter().fold(0.0f32, |a, b| a.max(*b)) as f64;
        let (a, _) = worst(&got, &want);
        println!("{name}: worst abs {a:.3e} ({:.3e} of peak {peak:.3})", a / peak);
        assert!(a / peak < 1e-5, "{name} differs from librosa: {:.3e} of peak", a / peak);
    }
}

#[test]
fn tempo_internals_match_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let hop = f.doc["hop"].as_u64().unwrap() as usize;
    let win_length = f.doc["win_length"].as_u64().unwrap() as usize;

    assert_eq!(
        nv_dsp::tempo::next_fast_len(2 * win_length - 1),
        f.doc["n_pad"].as_u64().unwrap() as usize,
        "next_fast_len"
    );
    assert_eq!(((8.0 * sr) as usize) / hop, win_length, "win_length");

    let bpms = nv_dsp::tempo::tempo_frequencies(win_length, sr, hop);
    let want_bpms = floats(&f.doc["bpms"]);
    for k in 1..win_length {
        let d = (bpms[k] - want_bpms[k] as f64).abs() / want_bpms[k] as f64;
        assert!(d < 1e-6, "bpms[{k}] differs: {} vs {}", bpms[k], want_bpms[k]);
    }

    let env = floats(&f.doc["env_median"]);
    let tg = nv_dsp::tempo::tempogram(&env, win_length);
    assert_eq!(tg.len(), f.doc["tempogram_n_cols"].as_u64().unwrap() as usize, "tempogram columns");

    let idx: Vec<usize> = f.doc["tempogram_col_idx"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    let mut abs = 0.0f64;
    for (k, &c) in idx.iter().enumerate() {
        let (a, _) = worst(&tg[c], &floats(&f.doc["tempogram_cols"][k]));
        abs = abs.max(a);
    }
    println!("tempogram: worst abs {abs:.3e}");
    assert!(abs < 1e-5, "tempogram differs from librosa: {abs:.3e}");

    // the time-averaged tempogram, which is what the prior is applied to
    let mut agg = vec![0.0f64; win_length];
    for col in &tg {
        for (a, v) in agg.iter_mut().zip(col) {
            *a += *v as f64;
        }
    }
    let want_agg = floats(&f.doc["tg_aggregated"]);
    let mut aabs = 0.0f64;
    for (k, a) in agg.iter().enumerate() {
        aabs = aabs.max((a / tg.len() as f64 - want_agg[k] as f64).abs());
    }
    println!("tempogram aggregated: worst abs {aabs:.3e}");
    assert!(aabs < 1e-6, "aggregated tempogram differs: {aabs:.3e}");
}

#[test]
fn tempo_matches_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let hop = f.doc["hop"].as_u64().unwrap() as usize;
    let env = floats(&f.doc["env_median"]);
    let got = nv_dsp::estimate_tempo(&env, sr, hop);
    let want = f.doc["tempo"].as_f64().unwrap();
    println!("tempo: {got:.6} vs librosa {want:.6}");
    // the estimate is one of a discrete set of lags, so it must land on
    // exactly the same one
    assert!((got - want).abs() < 1e-9, "tempo differs: {got} vs {want}");
}

#[test]
fn beat_track_matches_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let hop = f.doc["hop"].as_u64().unwrap() as usize;
    let env = floats(&f.doc["env_median"]);

    let (bpm, frames) = nv_dsp::beat_track(&env, sr, hop);
    let want: Vec<usize> = f.doc["beat_frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();

    println!("beat_track: {} beats vs librosa {}", frames.len(), want.len());
    println!("  got  {:?}", frames);
    println!("  want {:?}", want);
    assert!((bpm - f.doc["tempo"].as_f64().unwrap()).abs() < 1e-9, "tempo differs");
    // beat positions are frame indices: a one-frame shift is a real
    // behavioural difference, so these must match exactly
    assert_eq!(frames, want, "beat frames differ from librosa");
}

#[test]
fn onset_detect_matches_librosa() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let hop = f.doc["hop"].as_u64().unwrap() as usize;

    let p = nv_dsp::PeakPicker::for_onset_detect(sr, hop);
    println!(
        "peak picker: pre_max {} post_max {} pre_avg {} post_avg {} wait {}",
        p.pre_max, p.post_max, p.pre_avg, p.post_avg, p.wait
    );

    let env = floats(&f.doc["env_mean"]);
    let got = nv_dsp::onset_detect(&env, sr, hop);
    let want: Vec<usize> = f.doc["onset_frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    println!("onset_detect: {} onsets vs librosa {}", got.len(), want.len());
    assert_eq!(got, want, "onset frames differ from librosa");
}

fn sections(v: &Value) -> Vec<nv_dsp::filter::Section> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let c: Vec<f64> = r.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
            [c[0], c[1], c[2], c[3], c[4], c[5]]
        })
        .collect()
}

#[test]
fn butter_design_matches_scipy() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    for (name, cutoff, btype) in [
        ("sos_low", vec![220.0], nv_dsp::BandType::Low),
        ("sos_high", vec![4000.0], nv_dsp::BandType::High),
        ("sos_band", vec![220.0, 4000.0], nv_dsp::BandType::Band),
    ] {
        let got = nv_dsp::butter(4, &cutoff, btype, sr);
        let want = sections(&f.doc[name]);
        assert_eq!(got.len(), want.len(), "{name}: section count");
        let mut abs = 0.0f64;
        for (g, w) in got.iter().zip(&want) {
            for k in 0..6 {
                abs = abs.max((g[k] - w[k]).abs());
            }
        }
        println!("{name}: worst coefficient deviation {abs:.3e}");
        assert!(abs < 1e-9, "{name} differs from scipy: {abs:.3e}");
    }
}

#[test]
fn sosfiltfilt_matches_scipy() {
    let f = fixture();
    let sr = f.doc["sr"].as_f64().unwrap();
    let stride = f.doc["filt_stride"].as_u64().unwrap() as usize;
    let x: Vec<f64> = f.y.iter().map(|v| *v as f64).collect();

    for (name, cutoff, btype) in [
        ("filt_low", vec![220.0], nv_dsp::BandType::Low),
        ("filt_high", vec![4000.0], nv_dsp::BandType::High),
        ("filt_band", vec![220.0, 4000.0], nv_dsp::BandType::Band),
    ] {
        let sos = nv_dsp::butter(4, &cutoff, btype, sr);
        let got = nv_dsp::sosfiltfilt(&sos, &x);
        assert_eq!(got.len(), x.len(), "{name}: length");
        let want: Vec<f64> =
            f.doc[name].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        let mut abs = 0.0f64;
        for (k, w) in want.iter().enumerate() {
            abs = abs.max((got[k * stride] - w).abs());
        }
        println!("{name}: worst abs {abs:.3e}");
        assert!(abs < 1e-9, "{name} differs from scipy: {abs:.3e}");
    }
}

#[test]
fn median_filter_matches_scipy() {
    let f = fixture();
    let src: Vec<Vec<f32>> = f.doc["median_src"].as_array().unwrap().iter().map(floats).collect();

    for (name, horizontal) in [("median_h", true), ("median_p", false)] {
        let got = nv_dsp::median_filter_1d(&src, 17, horizontal);
        let want: Vec<Vec<f32>> = f.doc[name].as_array().unwrap().iter().map(floats).collect();
        let mut abs = 0.0f64;
        for (g, w) in got.iter().zip(&want) {
            let (a, _) = worst(g, w);
            abs = abs.max(a);
        }
        println!("{name}: worst abs {abs:.3e}");
        assert!(abs < 1e-9, "{name} differs from scipy: {abs:.3e}");
    }
}

#[test]
fn time_stretch_matches_librosa() {
    let f = fixture();
    let n_fft = f.doc["n_fft"].as_u64().unwrap() as usize;
    let hop = f.doc["hop"].as_u64().unwrap() as usize;

    for rate_v in f.doc["stretch_rates"].as_array().unwrap() {
        let rate = rate_v.as_f64().unwrap();
        let key = if rate == 0.8 { "0.8" } else { "1.25" };
        let got = nv_dsp::time_stretch(&f.y, rate, n_fft, hop);

        let want_len = f.doc["stretch_len"][key].as_u64().unwrap() as usize;
        assert_eq!(got.len(), want_len, "rate {rate}: length");

        let want = floats(&f.doc["stretch_samples"][key]);
        let off = got.len() / 3;
        let (abs, _) = worst(&got[off..off + want.len()], &want);
        let rms =
            (got.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>() / got.len() as f64).sqrt();
        let want_rms = f.doc["stretch_rms"][key].as_f64().unwrap();
        println!("time_stretch {rate}: worst abs {abs:.3e}, rms {rms:.6} vs {want_rms:.6}");
        assert!(abs < 1e-4, "time_stretch {rate} differs from librosa: {abs:.3e}");
        assert!((rms - want_rms).abs() / want_rms < 1e-3, "time_stretch {rate} rms differs");
    }
}
