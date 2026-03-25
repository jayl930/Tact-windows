use serde::{Deserialize, Serialize};

/// A speaker segment from diarization: which speaker was talking when.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    pub speaker_id: u32,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Result of diarization: speaker segments and number of detected speakers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizationResult {
    pub segments: Vec<SpeakerSegment>,
    pub speaker_count: u32,
}

/// Run speaker diarization on 16kHz mono PCM samples using pyannote-rs.
///
/// This runs on the original (untrimmed) audio to get accurate speaker timestamps.
/// pyannote-rs works with i16 samples, so we convert from f32.
pub fn diarize(samples: &[f32], sample_rate: u32) -> Result<DiarizationResult, String> {
    use pyannote_rs::{EmbeddingExtractor, EmbeddingManager};

    let duration_s = samples.len() as f64 / sample_rate as f64;
    tracing::info!("Starting diarization of {:.1}s audio", duration_s);

    // Convert f32 samples to i16 for pyannote-rs
    let samples_i16: Vec<i16> = samples
        .iter()
        .map(|&s| (s * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
        .collect();

    // Get model paths from the app's data directory
    let models_dir = crate::settings::settings_dir().join("models");

    let seg_model = models_dir.join("segmentation-3.0.onnx");
    let emb_model = models_dir.join("wespeaker_en_voxceleb_CAM++.onnx");

    if !seg_model.exists() || !emb_model.exists() {
        return Err(format!(
            "Diarization models not found. Please download:\n\
             - segmentation-3.0.onnx\n\
             - wespeaker_en_voxceleb_CAM++.onnx\n\
             to: {}",
            models_dir.display()
        ));
    }

    // Run pyannote-rs segmentation
    let segment_iter = pyannote_rs::get_segments(&samples_i16, sample_rate, &seg_model)
        .map_err(|e| format!("Segmentation failed: {}", e))?;

    let raw_segments: Vec<pyannote_rs::Segment> = segment_iter
        .filter_map(|r| r.ok())
        .collect();

    if raw_segments.is_empty() {
        tracing::info!("No speech segments detected by diarization");
        return Ok(DiarizationResult {
            segments: vec![],
            speaker_count: 0,
        });
    }

    // Extract embeddings and assign speakers
    let mut extractor = EmbeddingExtractor::new(&emb_model)
        .map_err(|e| format!("Embedding extractor init failed: {}", e))?;

    let mut manager = EmbeddingManager::new(10); // max 10 speakers

    let mut speaker_segments = Vec::new();
    let speaker_threshold = 0.5;

    for seg in &raw_segments {
        // pyannote Segment has .samples field with i16 data
        if seg.samples.len() < (sample_rate as usize / 4) {
            continue; // Skip very short segments (< 0.25s)
        }

        let embedding: Vec<f32> = match extractor.compute(&seg.samples) {
            Ok(iter) => iter.collect(),
            Err(e) => {
                tracing::warn!("Embedding failed for segment: {}", e);
                continue;
            }
        };

        // Try to match with existing speakers or create new one
        let speaker_id = match manager.search_speaker(embedding.clone(), speaker_threshold) {
            Some(id) => id,
            None => {
                // get_best_speaker_match will add the embedding if no match
                manager
                    .get_best_speaker_match(embedding)
                    .unwrap_or(0)
            }
        };

        speaker_segments.push(SpeakerSegment {
            speaker_id: speaker_id as u32,
            start_ms: (seg.start * 1000.0) as u64,
            end_ms: (seg.end * 1000.0) as u64,
        });
    }

    // Merge consecutive segments from the same speaker
    let merged = merge_speaker_segments(&speaker_segments);
    let speaker_count = count_unique_speakers(&merged);

    tracing::info!(
        "Diarization complete: {} segments, {} speakers",
        merged.len(),
        speaker_count
    );

    Ok(DiarizationResult {
        segments: merged,
        speaker_count,
    })
}

fn merge_speaker_segments(segments: &[SpeakerSegment]) -> Vec<SpeakerSegment> {
    if segments.is_empty() {
        return vec![];
    }

    let mut merged = vec![segments[0].clone()];
    for seg in &segments[1..] {
        let last = merged.last_mut().unwrap();
        if seg.speaker_id == last.speaker_id
            && seg.start_ms.saturating_sub(last.end_ms) <= 500
        {
            last.end_ms = seg.end_ms;
        } else {
            merged.push(seg.clone());
        }
    }
    merged
}

fn count_unique_speakers(segments: &[SpeakerSegment]) -> u32 {
    let mut seen = std::collections::HashSet::new();
    for seg in segments {
        seen.insert(seg.speaker_id);
    }
    seen.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_speaker_segments() {
        let segments = vec![
            SpeakerSegment { speaker_id: 0, start_ms: 0, end_ms: 1000 },
            SpeakerSegment { speaker_id: 0, start_ms: 1100, end_ms: 2000 },
            SpeakerSegment { speaker_id: 1, start_ms: 2500, end_ms: 3500 },
            SpeakerSegment { speaker_id: 1, start_ms: 3600, end_ms: 5000 },
        ];
        let merged = merge_speaker_segments(&segments);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].speaker_id, 0);
        assert_eq!(merged[0].end_ms, 2000);
        assert_eq!(merged[1].speaker_id, 1);
        assert_eq!(merged[1].end_ms, 5000);
    }
}
