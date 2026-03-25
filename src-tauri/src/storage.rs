use crate::settings;
use std::fs;
use std::time::{Duration, SystemTime};

/// Clean up old recordings based on the retention setting.
/// Only deletes audio files, not transcripts.
pub fn cleanup_old_recordings(retention_days: Option<u32>) {
    let days = match retention_days {
        Some(d) => d,
        None => return, // "Keep forever"
    };

    let recordings_dir = settings::settings_dir().join("recordings");
    if !recordings_dir.exists() {
        return;
    }

    let cutoff = SystemTime::now() - Duration::from_secs(days as u64 * 86400);

    let entries = match fs::read_dir(&recordings_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to read recordings dir: {}", e);
            return;
        }
    };

    let mut deleted = 0u32;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only delete audio files
        let is_audio = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e, "wav" | "ogg" | "opus" | "mp3" | "m4a" | "flac"))
            .unwrap_or(false);

        if !is_audio {
            continue;
        }

        // Check file age
        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };

        if modified < cutoff {
            if let Err(e) = fs::remove_file(&path) {
                tracing::warn!("Failed to delete {}: {}", path.display(), e);
            } else {
                deleted += 1;
            }
        }
    }

    if deleted > 0 {
        tracing::info!("Cleaned up {} old recording(s) (>{} days)", deleted, days);
    }
}
