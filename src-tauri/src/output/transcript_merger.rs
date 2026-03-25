use crate::diarization::pyannote::SpeakerSegment;
use crate::transcription::api_client::TranscriptionSegment;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A merged segment with speaker label, text, and timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedSegment {
    pub speaker: String,
    pub start: f64,
    pub end: f64,
    pub text: String,
}

/// Merge API transcription segments with diarization speaker segments.
///
/// For each transcription segment, finds the diarization segment with the
/// maximum time overlap and assigns that speaker.
pub fn merge(
    transcript_segments: &[TranscriptionSegment],
    speaker_segments: &[SpeakerSegment],
    speaker_names: &HashMap<u32, String>,
) -> Vec<MergedSegment> {
    transcript_segments
        .iter()
        .map(|ts| {
            let speaker = find_best_speaker(ts, speaker_segments, speaker_names);
            MergedSegment {
                speaker,
                start: ts.start,
                end: ts.end,
                text: ts.text.clone(),
            }
        })
        .collect()
}

/// Find the speaker with maximum overlap for a given transcription segment.
fn find_best_speaker(
    segment: &TranscriptionSegment,
    speaker_segments: &[SpeakerSegment],
    speaker_names: &HashMap<u32, String>,
) -> String {
    let seg_start_ms = (segment.start * 1000.0) as u64;
    let seg_end_ms = (segment.end * 1000.0) as u64;

    let mut best_overlap: u64 = 0;
    let mut best_speaker_id: Option<u32> = None;

    for ss in speaker_segments {
        let overlap_start = seg_start_ms.max(ss.start_ms);
        let overlap_end = seg_end_ms.min(ss.end_ms);

        if overlap_end > overlap_start {
            let overlap = overlap_end - overlap_start;
            if overlap > best_overlap {
                best_overlap = overlap;
                best_speaker_id = Some(ss.speaker_id);
            }
        }
    }

    match best_speaker_id {
        Some(id) => speaker_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Speaker {}", id + 1)),
        None => "Unknown".to_string(),
    }
}

/// Format merged segments as Markdown with speaker labels.
pub fn format_merged_transcript(segments: &[MergedSegment], duration: f64) -> String {
    let mut content = String::new();
    let mut last_speaker = String::new();

    for seg in segments {
        let timestamp = format_timestamp(seg.start);

        if seg.speaker != last_speaker {
            if !last_speaker.is_empty() {
                content.push('\n');
            }
            content.push_str(&format!("**{}**\n\n", seg.speaker));
            last_speaker = seg.speaker.clone();
        }

        content.push_str(&format!("{} {}\n\n", timestamp, seg.text));
    }

    content.push_str(&format!("---\n*Duration: {:.0}s*\n", duration));

    content
}

fn format_timestamp(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("[{:02}:{:02}:{:02}]", hours, minutes, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_best_speaker() {
        let segment = TranscriptionSegment {
            start: 1.0,
            end: 3.0,
            text: "Hello world".to_string(),
        };

        let speaker_segments = vec![
            SpeakerSegment { speaker_id: 0, start_ms: 0, end_ms: 2000 },
            SpeakerSegment { speaker_id: 1, start_ms: 2000, end_ms: 4000 },
        ];

        let mut names = HashMap::new();
        names.insert(0, "Alice".to_string());
        names.insert(1, "Bob".to_string());

        // Segment 1.0-3.0s overlaps with speaker 0 (1.0-2.0 = 1000ms) and speaker 1 (2.0-3.0 = 1000ms)
        // Both have equal overlap, so the first one found wins
        let speaker = find_best_speaker(&segment, &speaker_segments, &names);
        assert!(speaker == "Alice" || speaker == "Bob");
    }

    #[test]
    fn test_merge_assigns_speakers() {
        let transcript = vec![
            TranscriptionSegment { start: 0.0, end: 2.0, text: "Hello".to_string() },
            TranscriptionSegment { start: 2.5, end: 4.0, text: "Hi there".to_string() },
        ];

        let speakers = vec![
            SpeakerSegment { speaker_id: 0, start_ms: 0, end_ms: 2500 },
            SpeakerSegment { speaker_id: 1, start_ms: 2500, end_ms: 5000 },
        ];

        let mut names = HashMap::new();
        names.insert(0, "Alice".to_string());
        names.insert(1, "Bob".to_string());

        let merged = merge(&transcript, &speakers, &names);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].speaker, "Alice");
        assert_eq!(merged[1].speaker, "Bob");
    }
}
