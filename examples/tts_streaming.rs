//! Streaming TTS Example - True Streaming Architecture
//!
//! Demonstrates true streaming TTS where:
//! 1. LLM streams tokens in real-time
//! 2. TextBuffer accumulates characters and detects <stop/> markers
//! 3. TTS synthesis happens asynchronously
//! 4. Audio plays sequentially as segments are synthesized
//!
//! Usage:
//!     cargo run --example tts_streaming -- "你的问题"
//!
//! Environment variables:
//!     MINIMAX_API_KEY - Your MiniMax API key (required)
//!     OPENAI_API_KEY - Your OpenAI API key (required)
//!     OPENAI_BASE_URL - Your OpenAI base URL (required)
//!     LLM_MODEL - Model name (required)

use anyhow::Result;
use chat_assistant::audio::AudioPlayer;
use chat_assistant::llm::agent::LlmAgent;
use chat_assistant::tts::{start_tts_queue, TextBuffer};
use futures_util::StreamExt;
use std::env;

/// Clean punctuation from text for TTS (remove symbols that shouldn't be spoken)
fn clean_for_tts(text: &str) -> String {
    // Remove common punctuation that shouldn't be spoken
    let cleaned = text
        .replace("<stop/>", "")
        .replace("。", "")
        .replace("，", "")
        .replace("！", "")
        .replace("？", "")
        .replace("、", "")
        .replace("：", "")
        .replace("；", "")
        .replace("\"", "")
        .replace("\"", "")
        .replace("'", "")
        .replace("「", "")
        .replace("」", "")
        .replace("《", "")
        .replace("》", "")
        .replace("（", "")
        .replace("）", "")
        .replace("【", "")
        .replace("】", "")
        .trim()
        .to_string();
    // If result is empty, return original (to avoid silent TTS)
    if cleaned.is_empty() {
        text.trim().to_string()
    } else {
        cleaned
    }
}

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
    let tts = chat_assistant::tts::MiniMaxTtsHandler::new(
        minimax_key,
        "speech-2.8-hd".to_string(),
        "female-shaonv".to_string(),
        1.0f32,
        32000,
    );

    // Create audio player
    let player = AudioPlayer::new()?;

    // Start TTS queue (returns queue handle and sample receiver)
    let (tts_queue, mut sample_rx) = start_tts_queue(tts);

    // Create LLM agent with system prompt for speech
    let agent = LlmAgent::new(
        Some(&openai_key),
        Some(&openai_base_url),
        &default_model,
        &chat_assistant::llm::response::ResponseSegmenter::system_prompt(),
    )
    .await?;

    println!("Streaming TTS Example (True Streaming)");
    println!("====================================");
    println!("Input: {}", text);
    println!();

    // Create the token stream
    let mut token_stream = Box::pin(agent.stream_tokens(text));

    // Create text buffer
    let mut buffer = TextBuffer::new();

    // Collect all samples first, then play sequentially
    let mut all_samples: Vec<Vec<f32>> = Vec::new();

    println!("Streaming tokens...");

    // Process tokens and collect samples
    while let Some(token_result) = token_stream.next().await {
        match token_result {
            Ok(token) => {
                print!("{}", token);
                std::io::Write::flush(&mut std::io::stdout())?;

                // Push to buffer and check for segments
                if let Some(segment) = buffer.push(&token) {
                    let cleaned = clean_for_tts(&segment);
                    println!("\n[Segment detected]: {}", cleaned);
                    // Enqueue for TTS synthesis (async)
                    tts_queue.enqueue(cleaned).await?;
                }
            }
            Err(e) => {
                eprintln!("\nToken stream error: {}", e);
                break;
            }
        }

        // Try to collect samples
        while let Ok(Some(samples)) =
            tokio::time::timeout(tokio::time::Duration::from_millis(10), sample_rx.recv()).await
        {
            if !samples.is_empty() {
                all_samples.push(samples);
            }
        }
    }

    // Extract any remaining text in buffer
    if let Some(remaining) = buffer.extract_all() {
        if !remaining.is_empty() {
            let cleaned = clean_for_tts(&remaining);
            println!("\n[Final segment]: {}", cleaned);
            tts_queue.enqueue(cleaned).await?;
        }
    }

    // Drop the queue to signal completion
    drop(tts_queue);

    // Collect remaining samples
    while let Ok(Some(samples)) =
        tokio::time::timeout(tokio::time::Duration::from_secs(3), sample_rx.recv()).await
    {
        if !samples.is_empty() {
            all_samples.push(samples);
        }
    }

    // Now play all samples sequentially
    println!("\n\nPlaying {} audio segments...", all_samples.len());
    for (i, samples) in all_samples.iter().enumerate() {
        println!("Playing segment {}...", i + 1);
        player.play(samples)?;
        // Wait for this segment to finish before playing next
        player.wait();
    }

    println!("\nDone!");
    Ok(())
}
