use crate::transcription::api_client::TranscriptionResponse;
use std::path::Path;

/// Format seconds as [HH:MM:SS]
fn format_timestamp(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("[{:02}:{:02}:{:02}]", hours, minutes, secs)
}

/// Write a transcription response as a Markdown file with timestamps.
pub fn write_transcript(path: &Path, response: &TranscriptionResponse) -> Result<(), String> {
    let mut content = String::new();

    // Header
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Transcript");
    content.push_str(&format!("# {}\n\n", filename));

    if response.segments.is_empty() {
        // No segments — write the full text
        content.push_str(&response.text);
        content.push('\n');
    } else {
        for segment in &response.segments {
            content.push_str(&format!(
                "{} {}\n\n",
                format_timestamp(segment.start),
                segment.text
            ));
        }
    }

    // Write duration footer
    content.push_str(&format!(
        "---\n*Duration: {:.0}s*\n",
        response.duration
    ));

    std::fs::write(path, content).map_err(|e| format!("Failed to write transcript: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "[00:00:00]");
        assert_eq!(format_timestamp(65.0), "[00:01:05]");
        assert_eq!(format_timestamp(3661.0), "[01:01:01]");
    }
}
