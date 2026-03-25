use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the Claude CLI executable.
fn find_claude_cli() -> Option<PathBuf> {
    // Check PATH first
    let names = if cfg!(windows) {
        vec!["claude.exe", "claude.cmd", "claude.bat"]
    } else {
        vec!["claude"]
    };

    for name in &names {
        if let Ok(output) = Command::new(if cfg!(windows) { "where" } else { "which" })
            .arg(name)
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }

    // Check common install locations on Windows
    if cfg!(windows) {
        let common_paths = [
            r"C:\Users\%USERNAME%\AppData\Local\Programs\claude\claude.exe",
            r"C:\Program Files\Claude\claude.exe",
        ];
        for path_template in &common_paths {
            let path = if let Ok(username) = std::env::var("USERNAME") {
                path_template.replace("%USERNAME%", &username)
            } else {
                path_template.to_string()
            };
            let p = PathBuf::from(&path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // macOS/Linux common paths
    if !cfg!(windows) {
        let home = std::env::var("HOME").unwrap_or_default();
        let paths = [
            format!("{}/.local/bin/claude", home),
            format!("{}/.npm/bin/claude", home),
            "/usr/local/bin/claude".to_string(),
        ];
        for path in &paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Check if Claude CLI is available.
pub fn is_available() -> bool {
    find_claude_cli().is_some()
}

/// Run AI summary on a transcript file using Claude CLI.
pub fn generate_summary(
    transcript_path: &Path,
    destination: &str,
    output_folder: Option<&Path>,
) -> Result<PathBuf, String> {
    let claude_path = find_claude_cli()
        .ok_or("Claude CLI not found. Install it from https://claude.ai/download")?;

    let transcript_content =
        std::fs::read_to_string(transcript_path).map_err(|e| format!("Failed to read transcript: {}", e))?;

    let prompt = format!(
        "Summarize this meeting transcript concisely. Include key decisions, action items, and main topics discussed.\n\n{}",
        transcript_content
    );

    let output = Command::new(&claude_path)
        .arg("--print")
        .arg(&prompt)
        .output()
        .map_err(|e| format!("Failed to run Claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude CLI error: {}", stderr));
    }

    let summary = String::from_utf8_lossy(&output.stdout).to_string();

    // Determine output path
    let summary_path = match destination {
        "fixed" => {
            let folder = output_folder.ok_or("No output folder configured for fixed destination")?;
            let stem = transcript_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("summary");
            folder.join(format!("{}_summary.md", stem))
        }
        "subfolder" => {
            let parent = transcript_path.parent().unwrap_or(Path::new("."));
            let summaries_dir = parent.join("summaries");
            std::fs::create_dir_all(&summaries_dir).map_err(|e| e.to_string())?;
            let stem = transcript_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("summary");
            summaries_dir.join(format!("{}_summary.md", stem))
        }
        _ => {
            // "same" — write next to transcript
            let stem = transcript_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("summary");
            let parent = transcript_path.parent().unwrap_or(Path::new("."));
            parent.join(format!("{}_summary.md", stem))
        }
    };

    // Write summary
    let content = format!("# Summary\n\n{}\n", summary.trim());
    std::fs::write(&summary_path, content)
        .map_err(|e| format!("Failed to write summary: {}", e))?;

    tracing::info!("AI summary written to {}", summary_path.display());

    Ok(summary_path)
}
