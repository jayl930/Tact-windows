use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

const TARGET_SAMPLE_RATE: u32 = 16000;

/// Decode any supported audio file to 16kHz mono f32 PCM samples.
pub fn decode_to_pcm(path: &Path) -> Result<DecodedAudio, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Failed to probe audio format: {}", e))?;

    let mut format_reader = probed.format;

    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    let codec_params = track.codec_params.clone();
    let track_id = track.id;

    let source_sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let _source_channels = codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                tracing::warn!("Decode packet error: {}", e);
                break;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Decode error: {}", e);
                continue;
            }
        };

        let spec = *decoded.spec();
        let num_frames = decoded.capacity();

        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        let ch = spec.channels.count();

        // Mix to mono
        if ch == 1 {
            all_samples.extend_from_slice(interleaved);
        } else {
            for chunk in interleaved.chunks(ch) {
                let sum: f32 = chunk.iter().sum();
                all_samples.push(sum / ch as f32);
            }
        }
    }

    // Resample to target rate if needed
    let samples = if source_sample_rate != TARGET_SAMPLE_RATE {
        resample(&all_samples, source_sample_rate, TARGET_SAMPLE_RATE)?
    } else {
        all_samples
    };

    let duration = samples.len() as f64 / TARGET_SAMPLE_RATE as f64;

    Ok(DecodedAudio {
        samples,
        sample_rate: TARGET_SAMPLE_RATE,
        duration,
    })
}

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration: f64,
}

fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Result<Vec<f32>, String> {
    use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let ratio = to_rate as f64 / from_rate as f64;
    let chunk_size = 1024;

    let mut resampler = SincFixedIn::<f32>::new(
        ratio,
        2.0,
        params,
        chunk_size,
        1, // mono
    )
    .map_err(|e| format!("Resampler init error: {}", e))?;

    let mut output = Vec::new();

    for chunk in samples.chunks(chunk_size) {
        let input = if chunk.len() < chunk_size {
            let mut padded = chunk.to_vec();
            padded.resize(chunk_size, 0.0);
            vec![padded]
        } else {
            vec![chunk.to_vec()]
        };

        let resampled = resampler
            .process(&input, None)
            .map_err(|e| format!("Resample error: {}", e))?;

        if let Some(channel) = resampled.first() {
            output.extend_from_slice(channel);
        }
    }

    // Trim to expected length
    let expected_len = (samples.len() as f64 * ratio) as usize;
    output.truncate(expected_len);

    Ok(output)
}
