use crate::diarization::{pyannote, speaker_identifier};
use crate::output::{transcript_merger, transcript_writer};
use crate::recording::decoder;
use crate::transcription::api_client::{self, ApiProvider};
use crate::transcription::vad;
use std::path::{Path, PathBuf};

/// Full pipeline: decode → VAD → API transcription (+ diarization) → merge → write transcript
pub async fn transcribe_file(
    audio_path: &Path,
    api_provider: &str,
    api_key: &str,
    language: &str,
    output_folder: Option<&Path>,
    vad_enabled: bool,
    vad_threshold: f32,
    diarization_enabled: bool,
) -> Result<TranscriptionResult, String> {
    let provider = ApiProvider::from_str(api_provider);

    tracing::info!("Starting transcription of {}", audio_path.display());

    // Decode audio to 16kHz mono PCM for VAD and diarization
    let decoded = decoder::decode_to_pcm(audio_path)?;

    // VAD: trim silence if enabled
    let (api_audio_path, vad_mappings) = if vad_enabled {
        let vad_result = vad::process_vad(&decoded.samples, decoded.sample_rate, vad_threshold)?;

        if vad_result.trimmed_samples.is_empty() {
            return Err("No speech detected in recording".to_string());
        }

        // Write trimmed audio to temp WAV for API upload
        let temp_path = audio_path.with_extension("trimmed.wav");
        write_temp_wav(&temp_path, &vad_result.trimmed_samples, decoded.sample_rate)?;

        (temp_path, Some(vad_result.timestamp_mappings))
    } else {
        (audio_path.to_path_buf(), None)
    };

    // Run transcription API and diarization concurrently
    let api_future = api_client::transcribe(&provider, api_key, &api_audio_path, language);

    let diarization_result = if diarization_enabled {
        // Run diarization on original (untrimmed) audio
        match pyannote::diarize(&decoded.samples, decoded.sample_rate) {
            Ok(result) => Some(result),
            Err(e) => {
                tracing::warn!("Diarization failed (continuing without): {}", e);
                None
            }
        }
    } else {
        None
    };

    let mut response = api_future.await?;

    // Clean up temp trimmed file
    if vad_enabled {
        let _ = std::fs::remove_file(&api_audio_path);
    }

    // Re-map timestamps if VAD was used
    if let Some(mappings) = &vad_mappings {
        vad::remap_segments(&mut response.segments, mappings);
    }

    tracing::info!(
        "Transcription complete: {} segments, {:.1}s",
        response.segments.len(),
        response.duration
    );

    // Determine output path
    let output_dir = output_folder
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            audio_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        });

    let stem = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("transcript");

    let transcript_path = output_dir.join(format!("{}.md", stem));

    // Write transcript: with or without speaker labels
    let speaker_count = if let Some(diarization) = &diarization_result {
        // Load enrolled speakers for name assignment
        let db = speaker_identifier::SpeakerDatabase::load();
        let speaker_ids: Vec<u32> = diarization
            .segments
            .iter()
            .map(|s| s.speaker_id)
            .collect();
        let speaker_names =
            speaker_identifier::assign_speaker_names(&speaker_ids, &db.speakers);

        // Merge transcription + diarization
        let merged = transcript_merger::merge(
            &response.segments,
            &diarization.segments,
            &speaker_names,
        );

        // Write merged transcript with speaker labels
        let content = transcript_merger::format_merged_transcript(&merged, response.duration);
        let header = format!("# {}\n\n", stem);
        std::fs::write(&transcript_path, format!("{}{}", header, content))
            .map_err(|e| format!("Failed to write transcript: {}", e))?;

        diarization.speaker_count as usize
    } else {
        // Write plain transcript without speakers
        transcript_writer::write_transcript(&transcript_path, &response)?;
        0
    };

    tracing::info!("Transcript written to {}", transcript_path.display());

    // Post-processing: hooks (summary is handled in commands.rs with UI events)
    let settings = crate::settings::load_settings();

    if !settings.hooks.is_empty() {
        let hook_results = crate::automation::hooks::run_hooks(
            &settings.hooks,
            &transcript_path,
            audio_path,
            response.duration,
            speaker_count,
            &output_dir,
        );
        for result in &hook_results {
            if !result.success {
                tracing::warn!("Hook '{}' failed: {}", result.hook_name, result.stderr);
            }
        }
    }

    // Export audio alongside transcript if enabled
    if settings.export_audio && output_dir != audio_path.parent().unwrap_or(Path::new(".")) {
        let dest = output_dir.join(audio_path.file_name().unwrap_or_default());
        if let Err(e) = std::fs::copy(audio_path, &dest) {
            tracing::warn!("Audio export failed: {}", e);
        }
    }

    Ok(TranscriptionResult {
        transcript_path,
        audio_path: audio_path.to_path_buf(),
        duration: response.duration,
        segment_count: response.segments.len(),
        speaker_count,
    })
}

fn write_temp_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("WAV write error: {}", e))?;
    for &s in samples {
        writer
            .write_sample(s)
            .map_err(|e| format!("WAV sample error: {}", e))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("WAV finalize error: {}", e))?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptionResult {
    pub transcript_path: PathBuf,
    pub audio_path: PathBuf,
    pub duration: f64,
    pub segment_count: usize,
    pub speaker_count: usize,
}
