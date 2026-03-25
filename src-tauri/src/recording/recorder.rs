use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::settings;

const TARGET_SAMPLE_RATE: u32 = 48000;
const TARGET_CHANNELS: u16 = 1;

pub struct Recorder {
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    is_recording: Arc<AtomicBool>,
    start_time: Option<Instant>,
    device_sample_rate: u32,
    device_channels: u16,
}

impl Recorder {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(AtomicBool::new(false)),
            start_time: None,
            device_sample_rate: TARGET_SAMPLE_RATE,
            device_channels: TARGET_CHANNELS,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Err("Already recording".to_string());
        }

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        let device_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        tracing::info!("Using input device: {}", device_name);

        let config = Self::pick_config(&device)?;
        self.device_sample_rate = config.sample_rate;
        self.device_channels = config.channels;

        tracing::info!(
            "Recording config: {}Hz, {} channels",
            self.device_sample_rate,
            self.device_channels
        );

        // Clear previous samples
        self.samples.lock().unwrap().clear();

        let samples = Arc::clone(&self.samples);
        let is_recording = Arc::clone(&self.is_recording);
        let channels = self.device_channels as usize;

        let err_fn = |err: cpal::StreamError| {
            tracing::error!("Audio stream error: {}", err);
        };

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_recording.load(Ordering::Relaxed) {
                        return;
                    }
                    let mut buf = samples.lock().unwrap();
                    if channels == 1 {
                        buf.extend_from_slice(data);
                    } else {
                        // Mix down to mono by averaging channels
                        for chunk in data.chunks(channels) {
                            let sum: f32 = chunk.iter().sum();
                            buf.push(sum / channels as f32);
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start stream: {}", e))?;

        self.is_recording.store(true, Ordering::Relaxed);
        self.start_time = Some(Instant::now());
        self.stream = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<RecordingResult, String> {
        if !self.is_recording.load(Ordering::Relaxed) {
            return Err("Not recording".to_string());
        }

        self.is_recording.store(false, Ordering::Relaxed);
        // Drop the stream to stop capture
        self.stream.take();

        let duration = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        let samples = {
            let mut buf = self.samples.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        // Write WAV file
        let recordings_dir = settings::settings_dir().join("recordings");
        std::fs::create_dir_all(&recordings_dir)
            .map_err(|e| format!("Failed to create recordings dir: {}", e))?;

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = format!("{}.wav", timestamp);
        let filepath = recordings_dir.join(&filename);

        write_wav(&filepath, &samples, self.device_sample_rate)?;

        tracing::info!(
            "Recording saved: {} ({:.1}s, {} samples)",
            filepath.display(),
            duration,
            samples.len()
        );

        Ok(RecordingResult {
            path: filepath,
            duration,
            sample_rate: self.device_sample_rate,
            sample_count: samples.len(),
        })
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn elapsed_secs(&self) -> f64 {
        if self.is_recording.load(Ordering::Relaxed) {
            self.start_time
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0)
        } else {
            0.0
        }
    }

    fn pick_config(device: &Device) -> Result<StreamConfig, String> {
        let supported = device
            .supported_input_configs()
            .map_err(|e| format!("Failed to get supported configs: {}", e))?;

        // Try to find a config with our target sample rate, preferring mono
        let mut best: Option<cpal::SupportedStreamConfigRange> = None;
        for cfg in supported {
            if cfg.sample_format() != cpal::SampleFormat::F32 {
                continue;
            }
            match &best {
                None => best = Some(cfg),
                Some(current) => {
                    // Prefer mono
                    if cfg.channels() < current.channels() {
                        best = Some(cfg);
                    }
                }
            }
        }

        let range = best.ok_or("No suitable audio config found")?;

        // Clamp target sample rate to supported range
        let rate = TARGET_SAMPLE_RATE
            .max(range.min_sample_rate())
            .min(range.max_sample_rate());

        Ok(StreamConfig {
            channels: range.channels(),
            sample_rate: rate,
            buffer_size: cpal::BufferSize::Default,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub path: PathBuf,
    pub duration: f64,
    pub sample_rate: u32,
    pub sample_count: usize,
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: TARGET_CHANNELS,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("WAV write error: {}", e))?;

    for &sample in samples {
        writer
            .write_sample(sample)
            .map_err(|e| format!("WAV sample write error: {}", e))?;
    }

    writer
        .finalize()
        .map_err(|e| format!("WAV finalize error: {}", e))?;

    Ok(())
}
