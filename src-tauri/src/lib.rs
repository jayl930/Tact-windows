mod automation;
mod commands;
mod diarization;
mod folders;
mod output;
mod recording;
mod settings;
mod state;
mod storage;
mod transcription;

use commands::{QueueState, RecorderState};
use recording::recorder::Recorder;
use state::AppState;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use transcription::queue::TranscriptionQueue;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    // Run recording cleanup on startup
    let startup_settings = settings::load_settings();
    storage::cleanup_old_recordings(startup_settings.recording_retention_days);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let toggle_shortcut = Shortcut::new(
                            Some(Modifiers::CONTROL | Modifiers::SHIFT),
                            Code::KeyR,
                        );
                        if shortcut == &toggle_shortcut {
                            tracing::info!("Global hotkey Ctrl+Shift+R pressed");
                            let _ = app.emit("hotkey-toggle-record", ());
                        }
                    }
                })
                .build(),
        )
        .manage(AppState::new())
        .manage(RecorderState(Mutex::new(
            Recorder::new().expect("Failed to initialize recorder"),
        )))
        .manage(QueueState(Mutex::new(TranscriptionQueue::load())))
        .setup(|app| {
            // Apply window vibrancy/transparency
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                    let _ = apply_vibrancy(&window, NSVisualEffectMaterial::UnderWindowBackground, None, None);
                }
                #[cfg(target_os = "windows")]
                {
                    use window_vibrancy::apply_mica;
                    let _ = apply_mica(&window, Some(true)); // true = dark mode
                }
            }

            // Register global hotkey
            let shortcut =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyR);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                tracing::warn!("Failed to register global hotkey: {}", e);
            }

            // Build tray menu
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &separator, &quit])?;

            // Build tray icon
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Tact — Idle")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_app_state,
            commands::start_recording,
            commands::stop_recording,
            commands::get_recording_duration,
            commands::save_api_key,
            commands::get_api_key,
            commands::test_api_connection,
            commands::transcribe_file_cmd,
            commands::get_queue,
            commands::process_pending_queue,
            commands::retry_queue_item,
            commands::remove_queue_item,
            commands::get_enrolled_speakers,
            commands::enroll_speaker,
            commands::remove_enrolled_speaker,
            commands::check_claude_cli,
            commands::set_output_folder,
            commands::add_favorite_folder,
            commands::remove_favorite_folder,
            commands::pick_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
