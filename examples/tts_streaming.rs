//! Streaming TTS Example
//!
//! Demonstrates streaming text-to-speech where LLM output is processed
//! segment by segment, with each segment synthesized and played as it arrives.
//!
//! Usage:
//!     cargo run --example tts_streaming -- "你的问题"
//!
//! Environment variables:
//!     MINIMAX_API_KEY - Your MiniMax API key (required)
//!     OPENAI_API_KEY - Your OpenAI API key (required)

use anyhow::Result;
use chat_assistant::audio::AudioPlayer;
use chat_assistant::llm::agent::LlmAgent;
use chat_assistant::llm::ResponseSegmenter;
use chat_assistant::tts::MiniMaxTtsHandler;
use futures_util::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // Get API keys
    let minimax_key =
        env::var("MINIMAX_API_KEY").expect("MINIMAX_API_KEY environment variable not set");
    let openai_key =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let openai_base_url =
        env::var("OPENAI_BASE_URL").expect("OPENAI_BASE_URL environment variable not set");
    let default_model = env::var("LLM_MODEL").expect("LLM_MODEL environment variable not set");

    // Get text from command line arguments
    let text = env::args()
        .nth(1)
        .expect("Usage: cargo run --example tts_streaming -- \"Your question\"");

    // Create TTS handler
    let tts = MiniMaxTtsHandler::new(
        minimax_key,
        "speech-2.8-hd".to_string(),
        "male-qn-qingse".to_string(),
        1.0f32,
        32000,
    );

    // Create audio player
    let player = AudioPlayer::new()?;

    // Create LLM agent
    let agent = LlmAgent::new(
        Some(&openai_key),
        Some(&openai_base_url),
        &default_model,
        ResponseSegmenter::system_prompt(),
    )
    .await?;

    println!("Streaming TTS Example");
    println!("=====================");
    println!("Input: {}", text);
    println!();

    // Process the response
    let all_samples = process_streaming_response(&agent, &tts, &player, &text).await?;

    // Save all collected audio
    let output_path = "streaming_output.wav";
    if !all_samples.is_empty() {
        save_wav(&all_samples, 32000, output_path)?;
        println!("Audio saved to: {}", output_path);
    }

    Ok(())
}

/// Process LLM response stream, synthesizing and playing each segment
async fn process_streaming_response(
    agent: &LlmAgent,
    tts: &MiniMaxTtsHandler,
    player: &AudioPlayer,
    text: &str,
) -> Result<Vec<f32>> {
    // Get response from LLM (for now, we use the segmented approach)
    let segments = agent.stream_segments(text).await?;

    println!("Received {} segments", segments.len());

    let mut all_samples = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        println!("\n--- Segment {} ---", i + 1);
        println!("Text: {}", segment);

        // Stream TTS for this segment
        println!("Synthesizing...");
        let audio_stream = tts.synthesize_streaming(segment);
        let mut audio_stream = Box::pin(audio_stream);

        let mut total_samples = 0;
        let mut chunk_count = 0;

        while let Some(sample_result) = audio_stream.next().await {
            match sample_result {
                Ok(samples) => {
                    total_samples += samples.len();
                    chunk_count += 1;
                    all_samples.extend_from_slice(&samples);
                    // Play each chunk as it arrives
                    player.play(&samples)?;
                }
                Err(e) => {
                    eprintln!("TTS error: {}", e);
                    break;
                }
            }
        }

        println!(
            "Played {} chunks, {} total samples",
            chunk_count, total_samples
        );
    }

    // Wait for playback to finish
    while player.is_playing() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\nPlayback complete!");

    Ok(all_samples)
}

/// Save audio samples as a WAV file
fn save_wav(samples: &[f32], sample_rate: i32, path: &str) -> Result<()> {
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
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
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
