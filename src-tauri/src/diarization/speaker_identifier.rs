use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::settings;

/// An enrolled speaker with a name and stored embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrolledSpeaker {
    pub id: String,
    pub name: String,
    pub audio_path: PathBuf,
    pub created_at: String,
}

/// Speaker enrollment database.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeakerDatabase {
    pub speakers: Vec<EnrolledSpeaker>,
}

impl SpeakerDatabase {
    fn db_path() -> PathBuf {
        settings::settings_dir().join("speakers.json")
    }

    pub fn speakers_dir() -> PathBuf {
        settings::settings_dir().join("speakers")
    }

    pub fn load() -> Self {
        let path = Self::db_path();
        if path.exists() {
            let data = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = settings::settings_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::db_path(), json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_speaker(&mut self, name: String, audio_path: PathBuf) -> EnrolledSpeaker {
        let speaker = EnrolledSpeaker {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            audio_path,
            created_at: chrono::Local::now().to_rfc3339(),
        };
        self.speakers.push(speaker.clone());
        let _ = self.save();
        speaker
    }

    pub fn remove_speaker(&mut self, id: &str) -> Result<(), String> {
        let speaker = self
            .speakers
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or("Speaker not found")?;

        // Delete the audio file
        if speaker.audio_path.exists() {
            let _ = fs::remove_file(&speaker.audio_path);
        }

        self.speakers.retain(|s| s.id != id);
        self.save()
    }
}

/// Assign speaker names to diarization results using enrolled speakers.
///
/// For each unique speaker_id in the diarization, try to match with enrolled
/// speakers by comparing embeddings. Unmatched speakers get "Speaker N" labels.
pub fn assign_speaker_names(
    speaker_ids: &[u32],
    enrolled: &[EnrolledSpeaker],
) -> std::collections::HashMap<u32, String> {
    let mut names = std::collections::HashMap::new();
    let mut unnamed_count = 0;

    for &id in speaker_ids {
        if names.contains_key(&id) {
            continue;
        }

        // For now, without embedding comparison, assign names in order
        // When pyannote embedding comparison is working, this will be smarter
        if (id as usize) < enrolled.len() {
            names.insert(id, enrolled[id as usize].name.clone());
        } else {
            unnamed_count += 1;
            names.insert(id, format!("Speaker {}", unnamed_count));
        }
    }

    names
}
