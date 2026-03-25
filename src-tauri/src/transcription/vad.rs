use serde::{Deserialize, Serialize};

/// A detected speech segment with start/end in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechSegment {
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Mapping entry for re-mapping trimmed timestamps back to original timeline.
#[derive(Debug, Clone)]
pub struct TimestampMapping {
    pub trimmed_start_ms: u64,
    pub original_start_ms: u64,
    pub duration_ms: u64,
}

/// Result of VAD processing: speech segments + trimmed audio + mapping.
pub struct VadResult {
    pub speech_segments: Vec<SpeechSegment>,
    pub trimmed_samples: Vec<f32>,
    pub timestamp_mappings: Vec<TimestampMapping>,
    pub original_duration_ms: u64,
    pub trimmed_duration_ms: u64,
}

/// Run Silero VAD on 16kHz mono PCM samples.
pub fn process_vad(
    samples: &[f32],
    sample_rate: u32,
    threshold: f32,
) -> Result<VadResult, String> {
    use voice_activity_detector::VoiceActivityDetector;

    let chunk_size = 512usize;
    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(sample_rate as i32)
        .chunk_size(chunk_size)
        .build()
        .map_err(|e| format!("Failed to create VAD: {}", e))?;

    let samples_per_ms = sample_rate as f64 / 1000.0;
    let original_duration_ms = (samples.len() as f64 / samples_per_ms) as u64;

    let mut speech_segments: Vec<SpeechSegment> = Vec::new();
    let mut in_speech = false;
    let mut speech_start_ms: u64 = 0;

    for (i, chunk) in samples.chunks(chunk_size).enumerate() {
        if chunk.len() < chunk_size {
            break;
        }

        let probability: f32 = vad.predict(chunk.iter().copied());

        let current_ms = (i * chunk_size) as f64 / samples_per_ms;

        if probability >= threshold && !in_speech {
            in_speech = true;
            speech_start_ms = current_ms as u64;
        } else if probability < threshold && in_speech {
            in_speech = false;
            let end_ms = current_ms as u64;
            if end_ms - speech_start_ms > 100 {
                speech_segments.push(SpeechSegment {
                    start_ms: speech_start_ms,
                    end_ms,
                });
            }
        }
    }

    if in_speech {
        speech_segments.push(SpeechSegment {
            start_ms: speech_start_ms,
            end_ms: original_duration_ms,
        });
    }

    let merged = merge_close_segments(&speech_segments, 300);

    // Build trimmed audio and timestamp mapping
    let mut trimmed_samples = Vec::new();
    let mut timestamp_mappings = Vec::new();
    let mut trimmed_offset_ms: u64 = 0;

    for seg in &merged {
        let start_sample = (seg.start_ms as f64 * samples_per_ms) as usize;
        let end_sample = ((seg.end_ms as f64 * samples_per_ms) as usize).min(samples.len());

        if start_sample < end_sample {
            trimmed_samples.extend_from_slice(&samples[start_sample..end_sample]);

            let duration_ms = seg.end_ms - seg.start_ms;
            timestamp_mappings.push(TimestampMapping {
                trimmed_start_ms: trimmed_offset_ms,
                original_start_ms: seg.start_ms,
                duration_ms,
            });
            trimmed_offset_ms += duration_ms;
        }
    }

    let trimmed_duration_ms = trimmed_offset_ms;

    tracing::info!(
        "VAD: {:.1}s original → {:.1}s trimmed ({} speech segments, {:.0}% reduction)",
        original_duration_ms as f64 / 1000.0,
        trimmed_duration_ms as f64 / 1000.0,
        merged.len(),
        if original_duration_ms > 0 {
            (1.0 - trimmed_duration_ms as f64 / original_duration_ms as f64) * 100.0
        } else {
            0.0
        }
    );

    Ok(VadResult {
        speech_segments: merged,
        trimmed_samples,
        timestamp_mappings,
        original_duration_ms,
        trimmed_duration_ms,
    })
}

fn merge_close_segments(segments: &[SpeechSegment], max_gap_ms: u64) -> Vec<SpeechSegment> {
    if segments.is_empty() {
        return vec![];
    }

    let mut merged = vec![segments[0].clone()];
    for seg in &segments[1..] {
        let last = merged.last_mut().unwrap();
        if seg.start_ms.saturating_sub(last.end_ms) <= max_gap_ms {
            last.end_ms = seg.end_ms;
        } else {
            merged.push(seg.clone());
        }
    }
    merged
}

/// Re-map a timestamp from trimmed-audio timeline to original timeline.
pub fn remap_timestamp(trimmed_time_ms: u64, mappings: &[TimestampMapping]) -> u64 {
    for mapping in mappings {
        let trimmed_end = mapping.trimmed_start_ms + mapping.duration_ms;
        if trimmed_time_ms >= mapping.trimmed_start_ms && trimmed_time_ms <= trimmed_end {
            let offset = trimmed_time_ms - mapping.trimmed_start_ms;
            return mapping.original_start_ms + offset;
        }
    }
    if let Some(last) = mappings.last() {
        let trimmed_end = last.trimmed_start_ms + last.duration_ms;
        let overshoot = trimmed_time_ms.saturating_sub(trimmed_end);
        return last.original_start_ms + last.duration_ms + overshoot;
    }
    trimmed_time_ms
}

/// Re-map all segment timestamps from trimmed to original timeline.
pub fn remap_segments(
    segments: &mut [crate::transcription::api_client::TranscriptionSegment],
    mappings: &[TimestampMapping],
) {
    for seg in segments.iter_mut() {
        let start_ms = (seg.start * 1000.0) as u64;
        let end_ms = (seg.end * 1000.0) as u64;
        seg.start = remap_timestamp(start_ms, mappings) as f64 / 1000.0;
        seg.end = remap_timestamp(end_ms, mappings) as f64 / 1000.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_close_segments() {
        let segments = vec![
            SpeechSegment { start_ms: 0, end_ms: 1000 },
            SpeechSegment { start_ms: 1200, end_ms: 2000 },
            SpeechSegment { start_ms: 3000, end_ms: 4000 },
        ];
        let merged = merge_close_segments(&segments, 300);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].end_ms, 2000);
        assert_eq!(merged[1].start_ms, 3000);
    }

    #[test]
    fn test_remap_timestamp() {
        let mappings = vec![
            TimestampMapping { trimmed_start_ms: 0, original_start_ms: 0, duration_ms: 2000 },
            TimestampMapping { trimmed_start_ms: 2000, original_start_ms: 3000, duration_ms: 2000 },
        ];

        assert_eq!(remap_timestamp(0, &mappings), 0);
        assert_eq!(remap_timestamp(1000, &mappings), 1000);
        assert_eq!(remap_timestamp(2000, &mappings), 2000); // boundary: end of first mapping
        assert_eq!(remap_timestamp(2001, &mappings), 3001); // just into second mapping
        assert_eq!(remap_timestamp(3000, &mappings), 4000);
    }
}
