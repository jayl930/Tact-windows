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

/// Generate summary text via Claude CLI.
fn generate_via_cli(prompt: &str, transcript: &str) -> Result<String, String> {
    let claude_path = find_claude_cli()
        .ok_or("Claude CLI not found. Install it from https://claude.ai/download")?;

    let full_prompt = format!("{}\n\n{}", prompt, transcript);

    let output = Command::new(&claude_path)
        .arg("--print")
        .arg(&full_prompt)
        .output()
        .map_err(|e| format!("Failed to run Claude CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Claude CLI error: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Generate summary text via OpenAI/Groq chat completions API.
async fn generate_via_api(
    provider: &str,
    api_key: &str,
    prompt: &str,
    transcript: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err(format!(
            "No API key configured for {}. Go to Settings > API to add your key.",
            provider
        ));
    }

    let (endpoint, model) = match provider {
        "openai" => (
            "https://api.openai.com/v1/chat/completions",
            "gpt-4o-mini",
        ),
        "groq" => (
            "https://api.groq.com/openai/v1/chat/completions",
            "llama-3.3-70b-versatile",
        ),
        _ => return Err(format!("Unknown API provider: {}", provider)),
    };

    let user_message = format!("{}\n\n{}", prompt, transcript);

    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "user", "content": user_message }
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No content in API response".to_string())
}

/// Determine the output path for the summary file.
fn summary_output_path(
    transcript_path: &Path,
    destination: &str,
    output_folder: Option<&Path>,
) -> Result<PathBuf, String> {
    let stem = transcript_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("summary");

    match destination {
        "fixed" => {
            let folder =
                output_folder.ok_or("No output folder configured for fixed destination")?;
            Ok(folder.join(format!("{}_summary.md", stem)))
        }
        "subfolder" => {
            let parent = transcript_path.parent().unwrap_or(Path::new("."));
            let summaries_dir = parent.join("summaries");
            std::fs::create_dir_all(&summaries_dir).map_err(|e| e.to_string())?;
            Ok(summaries_dir.join(format!("{}_summary.md", stem)))
        }
        _ => {
            // "same" — write next to transcript
            let parent = transcript_path.parent().unwrap_or(Path::new("."));
            Ok(parent.join(format!("{}_summary.md", stem)))
        }
    }
}

/// Run AI summary on a transcript file using the configured provider.
pub async fn generate_summary(
    transcript_path: &Path,
    destination: &str,
    output_folder: Option<&Path>,
    provider: &str,
    api_key: &str,
    prompt: &str,
) -> Result<PathBuf, String> {
    let transcript_content = std::fs::read_to_string(transcript_path)
        .map_err(|e| format!("Failed to read transcript: {}", e))?;

    let summary = match provider {
        "openai" | "groq" => {
            generate_via_api(provider, api_key, prompt, &transcript_content).await?
        }
        _ => {
            // "claude_cli" or fallback
            let p = prompt.to_string();
            let t = transcript_content.clone();
            tokio::task::spawn_blocking(move || generate_via_cli(&p, &t))
                .await
                .map_err(|e| format!("Task join error: {}", e))??
        }
    };

    let summary_path = summary_output_path(transcript_path, destination, output_folder)?;

    let content = format!("# Summary\n\n{}\n", summary.trim());
    std::fs::write(&summary_path, content)
        .map_err(|e| format!("Failed to write summary: {}", e))?;

    tracing::info!("AI summary written to {}", summary_path.display());

    Ok(summary_path)
}
