use crate::diarization::speaker_identifier::{EnrolledSpeaker, SpeakerDatabase};
use crate::recording::recorder::Recorder;
use crate::settings::{self, TactSettings};
use crate::state::{AppState, AppStateInner};
use crate::transcription::{api_client, queue, service};
use std::sync::Mutex;
use tauri::{Emitter, State};

pub struct RecorderState(pub Mutex<Recorder>);
pub struct QueueState(pub Mutex<queue::TranscriptionQueue>);

// ── Settings ──

#[tauri::command]
pub fn get_settings() -> Result<TactSettings, String> {
    Ok(settings::load_settings())
}

#[tauri::command]
pub fn save_settings(new_settings: TactSettings) -> Result<(), String> {
    settings::save_settings_to_disk(&new_settings)
}

// ── App State ──

#[tauri::command]
pub fn get_app_state(state: State<'_, AppState>) -> Result<AppStateInner, String> {
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    Ok(inner.clone())
}

// ── Recording ──

#[tauri::command]
pub fn start_recording(
    recorder: State<'_, RecorderState>,
    app_state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut rec = recorder.0.lock().map_err(|e| e.to_string())?;
    rec.start()?;

    {
        let mut state = app_state.inner.lock().map_err(|e| e.to_string())?;
        state.is_recording = true;
        state.current_duration = 0.0;
    }
    app_state.emit_state(&app);

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    recorder: State<'_, RecorderState>,
    app_state: State<'_, AppState>,
    queue_state: State<'_, QueueState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let result = {
        let mut rec = recorder.0.lock().map_err(|e| e.to_string())?;
        rec.stop()?
    };

    {
        let mut state = app_state.inner.lock().map_err(|e| e.to_string())?;
        state.is_recording = false;
        state.current_duration = 0.0;
    }
    app_state.emit_state(&app);

    let current_settings = settings::load_settings();
    let audio_path = result.path.clone();

    // Always enqueue
    let queue_item = {
        let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
        q.enqueue(audio_path.clone())
    };
    let _ = app.emit("queue-updated", ());

    if current_settings.transcription_timing == "immediately" {
        // Process immediately
        let result_json =
            process_queue_item(&queue_item.id, &current_settings, &app_state, &queue_state, &app)
                .await;
        return result_json;
    }

    // For "on_return" and "manual", just return the enqueued info
    Ok(serde_json::json!({
        "path": result.path.to_string_lossy(),
        "duration": result.duration,
        "transcribed": false,
        "reason": "queued"
    }))
}

#[tauri::command]
pub fn get_recording_duration(recorder: State<'_, RecorderState>) -> Result<f64, String> {
    let rec = recorder.0.lock().map_err(|e| e.to_string())?;
    Ok(rec.elapsed_secs())
}

// ── Queue ──

#[tauri::command]
pub fn get_queue(queue_state: State<'_, QueueState>) -> Result<Vec<queue::QueueItem>, String> {
    let q = queue_state.0.lock().map_err(|e| e.to_string())?;
    Ok(q.items.clone())
}

#[tauri::command]
pub async fn process_pending_queue(
    app_state: State<'_, AppState>,
    queue_state: State<'_, QueueState>,
    app: tauri::AppHandle,
) -> Result<u32, String> {
    let current_settings = settings::load_settings();
    let pending_ids: Vec<String> = {
        let q = queue_state.0.lock().map_err(|e| e.to_string())?;
        q.pending_items().iter().map(|i| i.id.clone()).collect()
    };

    let mut processed = 0u32;
    for id in pending_ids {
        let _ = process_queue_item(&id, &current_settings, &app_state, &queue_state, &app).await;
        processed += 1;
    }

    Ok(processed)
}

#[tauri::command]
pub async fn retry_queue_item(
    id: String,
    app_state: State<'_, AppState>,
    queue_state: State<'_, QueueState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    {
        let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
        q.retry(&id);
    }
    let _ = app.emit("queue-updated", ());

    let current_settings = settings::load_settings();
    process_queue_item(&id, &current_settings, &app_state, &queue_state, &app).await
}

#[tauri::command]
pub fn remove_queue_item(
    id: String,
    queue_state: State<'_, QueueState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
    q.remove(&id);
    let _ = app.emit("queue-updated", ());
    Ok(())
}

/// Internal: process a single queue item through the transcription pipeline.
async fn process_queue_item(
    item_id: &str,
    settings: &TactSettings,
    app_state: &State<'_, AppState>,
    queue_state: &State<'_, QueueState>,
    app: &tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let api_key = get_stored_api_key(&settings.api_provider)?;
    if api_key.is_empty() {
        let _ = app.emit(
            "transcription-error",
            "No API key configured. Go to Settings > API to add your key.",
        );
        return Ok(serde_json::json!({
            "transcribed": false,
            "reason": "no_api_key"
        }));
    }

    // Get item audio path
    let audio_path = {
        let q = queue_state.0.lock().map_err(|e| e.to_string())?;
        q.items
            .iter()
            .find(|i| i.id == item_id)
            .map(|i| i.audio_path.clone())
            .ok_or("Queue item not found")?
    };

    // Mark as processing
    {
        let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
        q.update_status(item_id, queue::QueueItemStatus::Processing);
    }
    let _ = app.emit("queue-updated", ());

    {
        let mut state = app_state.inner.lock().map_err(|e| e.to_string())?;
        state.is_transcribing = true;
    }
    app_state.emit_state(app);
    let _ = app.emit("transcription-started", ());

    // Run transcription pipeline
    let result = service::transcribe_file(
        &audio_path,
        &settings.api_provider,
        &api_key,
        &settings.language,
        settings
            .output_folder
            .as_ref()
            .map(|s| std::path::Path::new(s.as_str())),
        settings.vad_enabled,
        settings.vad_threshold,
        settings.diarization_enabled,
    )
    .await;

    {
        let mut state = app_state.inner.lock().map_err(|e| e.to_string())?;
        state.is_transcribing = false;
    }
    app_state.emit_state(app);

    match result {
        Ok(tr) => {
            {
                let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
                q.mark_completed(item_id, tr.transcript_path.clone(), tr.duration);
            }
            let _ = app.emit("queue-updated", ());
            let _ = app.emit("transcription-complete", &tr);

            Ok(serde_json::json!({
                "transcribed": true,
                "transcript_path": tr.transcript_path.to_string_lossy(),
                "duration": tr.duration,
                "segment_count": tr.segment_count,
                "speaker_count": tr.speaker_count
            }))
        }
        Err(e) => {
            {
                let mut q = queue_state.0.lock().map_err(|e2| e2.to_string())?;
                q.mark_failed(item_id, e.clone());
            }
            let _ = app.emit("queue-updated", ());
            let _ = app.emit("transcription-error", &e);

            Ok(serde_json::json!({
                "transcribed": false,
                "reason": e
            }))
        }
    }
}

// ── API Keys ──

#[tauri::command]
pub fn save_api_key(provider: String, key: String) -> Result<(), String> {
    let service = format!("tact-{}", provider.to_lowercase());
    let entry = keyring::Entry::new(&service, "api_key")
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(&key)
        .map_err(|e| format!("Failed to save API key: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn get_api_key(provider: String) -> Result<String, String> {
    get_stored_api_key(&provider)
}

fn get_stored_api_key(provider: &str) -> Result<String, String> {
    let service = format!("tact-{}", provider.to_lowercase());
    let entry = keyring::Entry::new(&service, "api_key")
        .map_err(|e| format!("Keyring error: {}", e))?;
    match entry.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => Ok(String::new()),
        Err(e) => Err(format!("Failed to get API key: {}", e)),
    }
}

#[tauri::command]
pub async fn test_api_connection(provider: String, key: String) -> Result<String, String> {
    let api_provider = api_client::ApiProvider::from_str(&provider);
    api_client::test_connection(&api_provider, &key).await
}

// ── Transcribe arbitrary file ──

#[tauri::command]
pub async fn transcribe_file_cmd(
    audio_path: String,
    app_state: State<'_, AppState>,
    queue_state: State<'_, QueueState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(&audio_path);
    if !path.exists() {
        return Err(format!("File not found: {}", audio_path));
    }

    let item_id = {
        let mut q = queue_state.0.lock().map_err(|e| e.to_string())?;
        let item = q.enqueue(path.to_path_buf());
        item.id
    };
    let _ = app.emit("queue-updated", ());

    let current_settings = settings::load_settings();
    process_queue_item(&item_id, &current_settings, &app_state, &queue_state, &app).await
}

// ── Speaker Enrollment ──

#[tauri::command]
pub fn get_enrolled_speakers() -> Result<Vec<EnrolledSpeaker>, String> {
    let db = SpeakerDatabase::load();
    Ok(db.speakers)
}

#[tauri::command]
pub fn enroll_speaker(
    name: String,
    recorder: State<'_, RecorderState>,
) -> Result<EnrolledSpeaker, String> {
    // Stop recording the enrollment sample
    let result = {
        let mut rec = recorder.0.lock().map_err(|e| e.to_string())?;
        rec.stop()?
    };

    // Move recording to speakers directory
    let speakers_dir = SpeakerDatabase::speakers_dir();
    std::fs::create_dir_all(&speakers_dir).map_err(|e| e.to_string())?;

    let dest = speakers_dir.join(result.path.file_name().unwrap());
    std::fs::rename(&result.path, &dest).map_err(|e| e.to_string())?;

    let mut db = SpeakerDatabase::load();
    let speaker = db.add_speaker(name, dest);
    Ok(speaker)
}

#[tauri::command]
pub fn remove_enrolled_speaker(id: String) -> Result<(), String> {
    let mut db = SpeakerDatabase::load();
    db.remove_speaker(&id)
}

// ── Automation ──

#[tauri::command]
pub fn check_claude_cli() -> Result<bool, String> {
    Ok(crate::automation::summary::is_available())
}

// ── Folders ──

#[tauri::command]
pub fn set_output_folder(folder: String) -> Result<(), String> {
    crate::folders::set_output_folder(&folder)
}

#[tauri::command]
pub fn add_favorite_folder(folder: String) -> Result<(), String> {
    crate::folders::add_favorite_folder(&folder)
}

#[tauri::command]
pub fn remove_favorite_folder(folder: String) -> Result<(), String> {
    crate::folders::remove_favorite_folder(&folder)
}

#[tauri::command]
pub async fn pick_folder() -> Result<Option<String>, String> {
    // Returns the default recordings directory as a fallback
    // Actual folder picker is done via tauri-plugin-dialog on the frontend
    let dir = crate::settings::settings_dir().join("recordings");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(Some(dir.to_string_lossy().to_string()))
}
