use crate::settings::HookConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub hook_name: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Execute all enabled hooks with TACT_* environment variables.
pub fn run_hooks(
    hooks: &[HookConfig],
    transcript_path: &Path,
    audio_path: &Path,
    duration: f64,
    speaker_count: usize,
    output_folder: &Path,
) -> Vec<HookResult> {
    let mut results = Vec::new();

    for hook in hooks {
        if !hook.enabled {
            continue;
        }

        let script_path = Path::new(&hook.script_path);
        if !script_path.exists() {
            results.push(HookResult {
                hook_name: hook.name.clone(),
                success: false,
                stdout: String::new(),
                stderr: format!("Script not found: {}", hook.script_path),
                exit_code: None,
            });
            continue;
        }

        tracing::info!("Running hook: {} ({})", hook.name, hook.script_path);

        let result = execute_hook(
            script_path,
            transcript_path,
            audio_path,
            duration,
            speaker_count,
            output_folder,
        );

        results.push(result.unwrap_or_else(|e| HookResult {
            hook_name: hook.name.clone(),
            success: false,
            stdout: String::new(),
            stderr: e,
            exit_code: None,
        }));
    }

    results
}

fn execute_hook(
    script_path: &Path,
    transcript_path: &Path,
    audio_path: &Path,
    duration: f64,
    speaker_count: usize,
    output_folder: &Path,
) -> Result<HookResult, String> {
    let hook_name = script_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine how to execute based on extension
    let (program, args): (String, Vec<String>) = match script_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "ps1" => (
            "powershell".to_string(),
            vec![
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                script_path.to_string_lossy().to_string(),
            ],
        ),
        "bat" | "cmd" => (
            "cmd".to_string(),
            vec!["/c".to_string(), script_path.to_string_lossy().to_string()],
        ),
        "sh" => (
            "sh".to_string(),
            vec![script_path.to_string_lossy().to_string()],
        ),
        _ => (script_path.to_string_lossy().to_string(), vec![]),
    };

    let output = Command::new(&program)
        .args(&args)
        .env("TACT_TRANSCRIPT_PATH", transcript_path.to_string_lossy().as_ref())
        .env("TACT_AUDIO_PATH", audio_path.to_string_lossy().as_ref())
        .env("TACT_DURATION", format!("{:.0}", duration))
        .env("TACT_SPEAKER_COUNT", speaker_count.to_string())
        .env("TACT_OUTPUT_FOLDER", output_folder.to_string_lossy().as_ref())
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", hook_name, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let success = output.status.success();

    if !success {
        tracing::warn!("Hook {} failed (exit {}): {}", hook_name, output.status, stderr);
    } else {
        tracing::info!("Hook {} completed successfully", hook_name);
    }

    Ok(HookResult {
        hook_name,
        success,
        stdout,
        stderr,
        exit_code: output.status.code(),
    })
}
