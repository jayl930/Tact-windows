use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub duration: f64,
}

pub enum ApiProvider {
    Groq,
    OpenAi,
}

impl ApiProvider {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => ApiProvider::OpenAi,
            _ => ApiProvider::Groq,
        }
    }

    fn endpoint(&self) -> &str {
        match self {
            ApiProvider::Groq => "https://api.groq.com/openai/v1/audio/transcriptions",
            ApiProvider::OpenAi => "https://api.openai.com/v1/audio/transcriptions",
        }
    }

    fn model(&self) -> &str {
        match self {
            ApiProvider::Groq => "whisper-large-v3-turbo",
            ApiProvider::OpenAi => "whisper-1",
        }
    }
}

/// Verbose JSON response from Whisper API (both Groq and OpenAI use the same format)
#[derive(Debug, Deserialize)]
struct WhisperVerboseResponse {
    text: String,
    segments: Option<Vec<WhisperSegment>>,
    duration: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

pub async fn transcribe(
    provider: &ApiProvider,
    api_key: &str,
    audio_path: &Path,
    language: &str,
) -> Result<TranscriptionResponse, String> {
    let client = reqwest::Client::new();

    let file_bytes = tokio::fs::read(audio_path)
        .await
        .map_err(|e| format!("Failed to read audio file: {}", e))?;

    let file_name = audio_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav")
        .to_string();

    let file_part = multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/wav")
        .map_err(|e| format!("MIME error: {}", e))?;

    let mut form = multipart::Form::new()
        .part("file", file_part)
        .text("model", provider.model().to_string())
        .text("response_format", "verbose_json")
        .text("timestamp_granularities[]", "segment");

    if !language.is_empty() {
        form = form.text("language", language.to_string());
    }

    let response = client
        .post(provider.endpoint())
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(match status.as_u16() {
            401 => "Invalid API key. Please check your key in Settings.".to_string(),
            413 => "Audio file too large. Maximum size is 25MB.".to_string(),
            429 => "Rate limit exceeded. Please wait and try again.".to_string(),
            _ => format!("API error ({}): {}", status, error_body),
        });
    }

    let whisper_response: WhisperVerboseResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let segments = whisper_response
        .segments
        .unwrap_or_default()
        .into_iter()
        .map(|s| TranscriptionSegment {
            start: s.start,
            end: s.end,
            text: s.text.trim().to_string(),
        })
        .collect();

    Ok(TranscriptionResponse {
        text: whisper_response.text,
        segments,
        duration: whisper_response.duration.unwrap_or(0.0),
    })
}

/// Test API connection with a minimal request
pub async fn test_connection(provider: &ApiProvider, api_key: &str) -> Result<String, String> {
    // Create a tiny silent WAV file for testing
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("tact_test.wav");

    // Write a minimal 1-second silent WAV
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(&test_file, spec).map_err(|e| format!("WAV error: {}", e))?;
    for _ in 0..16000 {
        writer.write_sample(0i16).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;

    let result = transcribe(provider, api_key, &test_file, "en").await;
    let _ = std::fs::remove_file(&test_file);

    match result {
        Ok(_) => Ok("Connection successful!".to_string()),
        Err(e) => Err(e),
    }
}
