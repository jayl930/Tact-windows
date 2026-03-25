use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::settings;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueueItemStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub audio_path: PathBuf,
    pub status: QueueItemStatus,
    pub created_at: DateTime<Local>,
    pub completed_at: Option<DateTime<Local>>,
    pub transcript_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub duration: Option<f64>,
}

impl QueueItem {
    pub fn new(audio_path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            audio_path,
            status: QueueItemStatus::Pending,
            created_at: Local::now(),
            completed_at: None,
            transcript_path: None,
            error_message: None,
            duration: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptionQueue {
    pub items: Vec<QueueItem>,
}

impl TranscriptionQueue {
    fn queue_path() -> PathBuf {
        settings::settings_dir().join("queue.json")
    }

    pub fn load() -> Self {
        let path = Self::queue_path();
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
        fs::write(Self::queue_path(), json).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn enqueue(&mut self, audio_path: PathBuf) -> QueueItem {
        let item = QueueItem::new(audio_path);
        self.items.push(item.clone());
        let _ = self.save();
        item
    }

    pub fn update_status(&mut self, id: &str, status: QueueItemStatus) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = status;
            let _ = self.save();
        }
    }

    pub fn mark_completed(
        &mut self,
        id: &str,
        transcript_path: PathBuf,
        duration: f64,
    ) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Completed;
            item.completed_at = Some(Local::now());
            item.transcript_path = Some(transcript_path);
            item.duration = Some(duration);
            let _ = self.save();
        }
    }

    pub fn mark_failed(&mut self, id: &str, error: String) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Failed;
            item.error_message = Some(error);
            let _ = self.save();
        }
    }

    pub fn retry(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Pending;
            item.error_message = None;
            let _ = self.save();
        }
    }

    pub fn remove(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
        let _ = self.save();
    }

    pub fn pending_items(&self) -> Vec<&QueueItem> {
        self.items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Pending)
            .collect()
    }

    pub fn has_pending(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.status == QueueItemStatus::Pending)
    }

    pub fn is_processing(&self) -> bool {
        self.items
            .iter()
            .any(|i| i.status == QueueItemStatus::Processing)
    }
}
