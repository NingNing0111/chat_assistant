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
    let minimax_key = env::var("MINIMAX_API_KEY")
        .expect("MINIMAX_API_KEY environment variable not set");
    let openai_key = env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY environment variable not set");

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
        None,
        "gpt-4o-mini",
        ResponseSegmenter::system_prompt(),
    )
    .await?;

    println!("Streaming TTS Example");
    println!("=====================");
    println!("Input: {}", text);
    println!();

    // Process the response
    process_streaming_response(&agent, &tts, &player, &text).await?;

    Ok(())
}

/// Process LLM response stream, synthesizing and playing each segment
async fn process_streaming_response(
    agent: &LlmAgent,
    tts: &MiniMaxTtsHandler,
    player: &AudioPlayer,
    text: &str,
) -> Result<()> {
    // Get response from LLM (for now, we use the segmented approach)
    let segments = agent.stream_segments(text).await?;

    println!("Received {} segments", segments.len());

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
                    // Play each chunk as it arrives
                    player.play(&samples)?;
                }
                Err(e) => {
                    eprintln!("TTS error: {}", e);
                    break;
                }
            }
        }

        println!("Played {} chunks, {} total samples", chunk_count, total_samples);
    }

    // Wait for playback to finish
    while player.is_playing() {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("\nPlayback complete!");
    Ok(())
}
