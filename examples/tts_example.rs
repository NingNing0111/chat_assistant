//! MiniMax TTS Example
//!
//! Demonstrates text-to-speech synthesis using MiniMax WebSocket API.
//!
//! Usage:
//!     cargo run --example tts_example -- "要合成的文本"
//!
//! Environment variables:
//!     MINIMAX_API_KEY - Your MiniMax API key (required)
//!     MINIMAX_TTS_MODEL - Model name (default: speech-2.8-hd)
//!     MINIMAX_TTS_VOICE - Voice ID (default: male-qn-qingse)
//!     MINIMAX_TTS_SPEED - Speech speed (default: 1.0)

use chat_assistant::tts::MiniMaxTtsHandler;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Get API key from environment
    let api_key =
        env::var("MINIMAX_API_KEY").expect("MINIMAX_API_KEY environment variable not set");

    // Optional configuration from environment
    let model = env::var("MINIMAX_TTS_MODEL").unwrap_or_else(|_| "speech-2.8-hd".to_string());
    let voice_id = env::var("MINIMAX_TTS_VOICE").unwrap_or_else(|_| "male-qn-qingse".to_string());
    let speed: f32 = env::var("MINIMAX_TTS_SPEED")
        .unwrap_or_else(|_| "1.0".to_string())
        .parse()
        .unwrap_or(1.0);
    let sample_rate = 32000;

    // Get text from command line arguments
    let text = env::args()
        .nth(1)
        .expect("Usage: cargo run --example tts_example -- \"Your text here\"");

    println!("MiniMax TTS Example");
    println!("===================");
    println!("Model: {}", model);
    println!("Voice: {}", voice_id);
    println!("Speed: {}", speed);
    println!("Sample rate: {}", sample_rate);
    println!();
    println!("Synthesizing: {}", text);
    println!();

    // Create TTS handler
    let handler = MiniMaxTtsHandler::new(api_key, model, voice_id, speed, sample_rate);

    // Synthesize speech
    println!("Connecting to MiniMax API...");
    let samples = handler.synthesize(&text).await?;
    println!("Received {} audio samples", samples.len());

    // Calculate audio duration
    let duration_secs = samples.len() as f32 / sample_rate as f32;
    println!("Audio duration: {:.2} seconds", duration_secs);

    // Save to WAV file for playback
    let output_path = "tts_output.wav";
    save_wav(&samples, sample_rate, output_path)?;
    println!("Audio saved to: {}", output_path);

    Ok(())
}

/// Save audio samples as a WAV file
fn save_wav(samples: &[f32], sample_rate: i32, path: &str) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::Write;

    if samples.is_empty() {
        anyhow::bail!("No samples to save");
    }

    let num_channels = 1;
    let bits_per_sample = 16;
    let byte_rate = sample_rate * num_channels * bits_per_sample / 8;
    let block_align = num_channels * bits_per_sample / 8;
    let data_size = samples.len() * 2; // i16 = 2 bytes

    let mut file = File::create(path)?;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_size as u32).to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt chunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // audio format (PCM)
    file.write_all(&num_channels.to_le_bytes())?;
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&block_align.to_le_bytes())?;
    file.write_all(&bits_per_sample.to_le_bytes())?;

    // data chunk
    file.write_all(b"data")?;
    file.write_all(&(data_size as u32).to_le_bytes())?;

    // Convert f32 samples to i16 and write
    for &sample in samples {
        let clipped = sample.max(-1.0).min(1.0);
        let int_sample = (clipped * i16::MAX as f32) as i16;
        file.write_all(&int_sample.to_le_bytes())?;
    }

    Ok(())
}
