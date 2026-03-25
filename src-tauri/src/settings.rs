use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TactSettings {
    pub language: String,
    pub transcription_timing: String,
    pub diarization_enabled: bool,
    pub recording_retention_days: Option<u32>,
    pub launch_at_login: bool,
    pub export_audio: bool,
    pub vad_enabled: bool,
    pub vad_threshold: f32,
    pub api_provider: String,
    pub output_folder: Option<String>,
    pub favorite_folders: Vec<String>,
    pub recent_folders: Vec<String>,
    // Automation
    pub ai_summary_enabled: bool,
    pub ai_summary_destination: String,
    pub hooks: Vec<HookConfig>,
    // Language picker customization
    pub enabled_languages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub id: String,
    pub name: String,
    pub script_path: String,
    pub enabled: bool,
}

impl Default for TactSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            transcription_timing: "immediately".to_string(),
            diarization_enabled: false,
            recording_retention_days: Some(30),
            launch_at_login: false,
            export_audio: false,
            vad_enabled: true,
            vad_threshold: 0.5,
            api_provider: "groq".to_string(),
            output_folder: None,
            favorite_folders: vec![],
            recent_folders: vec![],
            ai_summary_enabled: false,
            ai_summary_destination: "same".to_string(),
            hooks: vec![],
            enabled_languages: vec!["en".to_string(), "ko".to_string()],
        }
    }
}

pub fn settings_dir() -> PathBuf {
    ProjectDirs::from("com", "tact", "Tact")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

pub fn load_settings() -> TactSettings {
    let path = settings_path();
    if path.exists() {
        let data = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        let settings = TactSettings::default();
        let _ = save_settings_to_disk(&settings);
        settings
    }
}

pub fn save_settings_to_disk(settings: &TactSettings) -> Result<(), String> {
    let dir = settings_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(settings_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}
