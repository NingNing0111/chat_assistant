//! MiniMax TTS handler using HTTP API
//!
//! Sends text to MiniMax's HTTP API for text-to-speech synthesis.
//! Audio is returned as hex-encoded MP3 which is decoded to f32 samples.

use anyhow::{Context, Result};
use minimp3::{Decoder, Frame};
use reqwest::Client;
use std::io::Read;

/// MiniMax HTTP TTS handler
pub struct MiniMaxTtsHandler {
    client: Client,
    api_key: String,
    model: String,
    voice_id: String,
    speed: f32,
    sample_rate: i32,
}

impl MiniMaxTtsHandler {
    /// Create a new MiniMax TTS handler
    pub fn new(api_key: String, model: String, voice_id: String, speed: f32, sample_rate: i32) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            voice_id,
            speed,
            sample_rate,
        }
    }

    /// Synthesize text to audio samples
    pub async fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        let audio_data = self.request_audio(text).await?;
        let samples = self.decode_mp3(&audio_data)?;
        Ok(samples)
    }

    /// Request audio from MiniMax HTTP API
    async fn request_audio(&self, text: &str) -> Result<Vec<u8>> {
        let url = "https://api.minimaxi.com/v1/t2a_v2";

        let request_body = serde_json::json!({
            "model": self.model,
            "text": text,
            "stream": false,
            "voice_setting": {
                "voice_id": self.voice_id,
                "speed": self.speed,
                "vol": 1,
                "pitch": 0
            },
            "audio_setting": {
                "sample_rate": self.sample_rate,
                "bitrate": 128000,
                "format": "mp3",
                "channel": 1
            }
        });

        let response = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to MiniMax API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API request failed: {} - {}", status, body);
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse API response")?;

        // Check for API-level errors
        if let Some(base_resp) = response_body.get("base_resp") {
            if let Some(status_code) = base_resp.get("status_code").and_then(|c| c.as_i64()) {
                if status_code != 0 {
                    let msg = base_resp.get("status_msg")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    anyhow::bail!("API error {}: {}", status_code, msg);
                }
            }
        }

        // Extract hex audio from response
        let audio_hex = response_body
            .get("data")
            .and_then(|d| d.get("audio"))
            .and_then(|a| a.as_str())
            .context("No audio data in response")?;

        let audio_data = hex::decode(audio_hex)
            .context("Failed to decode audio hex")?;

        Ok(audio_data)
    }

    /// Decode MP3 bytes to f32 samples
    fn decode_mp3(&self, mp3_data: &[u8]) -> Result<Vec<f32>> {
        if mp3_data.is_empty() {
            return Ok(Vec::new());
        }

        let mut decoder = Decoder::new(std::io::Cursor::new(mp3_data));
        let mut samples = Vec::new();

        loop {
            match decoder.next_frame() {
                Ok(Frame { data, .. }) => {
                    // MP3 data is i16 samples, convert to f32
                    for &sample in data.iter() {
                        samples.push(sample as f32 / i16::MAX as f32);
                    }
                }
                Err(minimp3::Error::Eof) => break,
                Err(e) => {
                    tracing::warn!("MP3 decode error: {:?}", e);
                    break;
                }
            }
        }

        Ok(samples)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }
}
