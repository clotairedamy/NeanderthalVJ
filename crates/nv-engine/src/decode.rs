//! Decoding an audio file to interleaved stereo f32, via Symphonia.

use std::fs::File;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// A decoded track, as the engine holds it.
#[derive(Debug, Clone)]
pub struct Track {
    /// Interleaved stereo.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

impl Track {
    pub fn frames(&self) -> usize {
        self.samples.len().checked_div(self.channels).unwrap_or(0)
    }

    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.sample_rate.max(1) as f64
    }

    /// The mono mix the analysis reads.
    pub fn mono(&self) -> Vec<f32> {
        if self.channels <= 1 {
            return self.samples.clone();
        }
        self.samples
            .chunks_exact(self.channels)
            .map(|f| f.iter().sum::<f32>() / self.channels as f32)
            .collect()
    }
}

#[derive(Debug)]
pub enum DecodeError {
    Open(String),
    Unsupported(String),
    Decode(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open(e) => write!(f, "could not open the file: {e}"),
            Self::Unsupported(e) => write!(f, "unsupported audio: {e}"),
            Self::Decode(e) => write!(f, "could not decode: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Decode a whole file into memory.
///
/// The visuals need random access to the signal (the beat grid is built
/// over the whole track before playback starts), so this reads it all
/// rather than streaming.
pub fn load(path: impl AsRef<Path>) -> Result<Track, DecodeError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| DecodeError::Open(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| DecodeError::Unsupported("no audio track".into()))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let mut channels = track.codec_params.channels.map_or(2, |c| c.count());
    let mut buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            // the end of the stream arrives as an error from Symphonia
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(DecodeError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio) => {
                let spec = *audio.spec();
                sample_rate = spec.rate;
                channels = spec.channels.count();
                let b = buf
                    .get_or_insert_with(|| SampleBuffer::<f32>::new(audio.capacity() as u64, spec));
                b.copy_interleaved_ref(audio);
                samples.extend_from_slice(b.samples());
            }
            // a corrupt packet mid-file should not lose the whole track
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(DecodeError::Decode(e.to_string())),
        }
    }

    if samples.is_empty() {
        return Err(DecodeError::Decode("the file held no audio".into()));
    }
    Ok(Track { samples, sample_rate, channels })
}
