use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateInner {
    pub is_recording: bool,
    pub is_transcribing: bool,
    pub current_duration: f64,
    pub active_tab: String,
}

impl Default for AppStateInner {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_transcribing: false,
            current_duration: 0.0,
            active_tab: "record".to_string(),
        }
    }
}

pub struct AppState {
    pub inner: Mutex<AppStateInner>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppStateInner::default()),
        }
    }

    pub fn emit_state(&self, app: &AppHandle) {
        if let Ok(state) = self.inner.lock() {
            let _ = app.emit("app-state-changed", state.clone());
        }
    }
}
