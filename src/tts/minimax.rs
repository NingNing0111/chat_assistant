//! MiniMax TTS handler using HTTP API
//!
//! Sends text to MiniMax's HTTP API for text-to-speech synthesis.
//! Audio is returned as hex-encoded MP3 which is decoded to f32 samples.

use anyhow::{Context, Result};
use async_stream::stream;
use futures_util::StreamExt;
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

    /// Synthesize text to audio samples (non-streaming)
    pub async fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        let audio_data = self.request_audio(text, false).await?;
        let samples = self.decode_mp3(&audio_data)?;
        Ok(samples)
    }

    /// Stream audio chunks from MiniMax HTTP API (streaming mode)
    pub fn synthesize_streaming<'a>(&'a self, text: &'a str) -> impl StreamExt<Item = Result<Vec<f32>>> + 'a {
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let voice_id = self.voice_id.clone();
        let speed = self.speed;
        let sample_rate = self.sample_rate;

        stream! {
            let url = "https://api.minimaxi.com/v1/t2a_v2";

            let request_body = serde_json::json!({
                "model": model,
                "text": text,
                "stream": true,
                "voice_setting": {
                    "voice_id": voice_id,
                    "speed": speed,
                    "vol": 1,
                    "pitch": 0
                },
                "audio_setting": {
                    "sample_rate": sample_rate,
                    "bitrate": 128000,
                    "format": "mp3",
                    "channel": 1
                }
            });

            let response = client
                .post(url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .context("Failed to send streaming request")?;

            if !response.status().is_success() {
                yield Err(anyhow::anyhow!("Streaming request failed: {}", response.status()));
                return;
            }

            let mut stream = response.bytes_stream();

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        // SSE format: "data: {...}\n\n"
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            for line in text.lines() {
                                if let Some(json_str) = line.strip_prefix("data: ") {
                                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(json_str) {
                                        // Check for error in response
                                        if let Some(base_resp) = response.get("base_resp") {
                                            if let Some(status_code) = base_resp.get("status_code").and_then(|c| c.as_i64()) {
                                                if status_code != 0 {
                                                    let msg = base_resp.get("status_msg")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("Unknown error");
                                                    yield Err(anyhow::anyhow!("API error {}: {}", status_code, msg));
                                                    return;
                                                }
                                            }
                                        }

                                        // Extract audio chunk
                                        if let Some(audio_hex) = response
                                            .get("data")
                                            .and_then(|d| d.get("audio"))
                                            .and_then(|a| a.as_str())
                                        {
                                            if !audio_hex.is_empty() {
                                                match hex::decode(audio_hex) {
                                                    Ok(audio_data) => {
                                                        let samples = decode_mp3_chunk(&audio_data);
                                                        if !samples.is_empty() {
                                                            yield Ok(samples);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!("Failed to decode audio hex: {}", e);
                                                    }
                                                }
                                            }
                                        }

                                        // Check if this is the final chunk
                                        if response.get("data").and_then(|d| d.get("status")).and_then(|s| s.as_i64()) == Some(2) {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream error: {}", e));
                        return;
                    }
                }
            }
        }
    }

    /// Request audio from MiniMax HTTP API (non-streaming)
    async fn request_audio(&self, text: &str, _stream: bool) -> Result<Vec<u8>> {
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
        Ok(decode_mp3_chunk(mp3_data))
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }
}

/// Decode MP3 bytes to f32 samples (helper function)
fn decode_mp3_chunk(mp3_data: &[u8]) -> Vec<f32> {
    if mp3_data.is_empty() {
        return Vec::new();
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

    samples
}
