//! Wake Word Detection Example
//!
//! Demonstrates wake word detection using sherpa-onnx keyword spotter.
//! Listens to microphone and detects specified wake words.
//!
//! Usage:
//!     cargo run --example wakeup -- --encoder encoder.onnx --decoder decoder.onnx --joiner joiner.onnx --tokens tokens.txt --keywords keywords.txt
//!
//! Environment variables (optional):
//!     WAKEWORD_PROVIDER=cpu (or cuda, etc.)

use anyhow::Result;
use chat_assistant::wakeword::WakeWordDetector;
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the encoder model (.onnx)
    #[arg(long)]
    encoder: PathBuf,

    /// Path to the decoder model (.onnx)
    #[arg(long)]
    decoder: PathBuf,

    /// Path to the joiner model (.onnx)
    #[arg(long)]
    joiner: PathBuf,

    /// Path to the tokens file
    #[arg(long)]
    tokens: PathBuf,

    /// Path to the keywords file
    #[arg(long)]
    keywords: PathBuf,

    /// Provider to use (cpu, cuda, etc.)
    #[arg(long, default_value = "cpu")]
    provider: String,

    /// Sample rate for audio capture
    #[arg(long, default_value_t = 16000)]
    sample_rate: i32,

    /// Enable debug logging
    #[arg(long, default_value_t = false)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Wake Word Detection Example");
    println!("============================");
    println!("Models:");
    println!("  encoder: {}", args.encoder.display());
    println!("  decoder: {}", args.decoder.display());
    println!("  joiner:  {}", args.joiner.display());
    println!("  tokens:  {}", args.tokens.display());
    println!("  keywords: {}", args.keywords.display());
    println!("  provider: {}", args.provider);
    println!();

    // Create wake word detector
    println!("Initializing wake word detector...");
    let detector = WakeWordDetector::new(
        &args.encoder,
        &args.decoder,
        &args.joiner,
        &args.tokens,
        &args.keywords,
        &args.provider,
    )?;
    println!("Wake word detector ready!");
    println!();

    // Setup audio capture
    let sample_rate = args.sample_rate as u32;
    let (tx, rx) = channel();

    println!("Starting audio capture at {} Hz...", sample_rate);

    // Audio capture thread
    thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                eprintln!("No input device available");
                return;
            }
        };

        println!("Using input device: {}", device.name().unwrap_or_default());

        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get input config: {}", e);
                return;
            }
        };

        println!("Input config: {:?}", config);

        let stream = match device.build_input_stream(
            &config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if !data.is_empty() {
                    let _ = tx.send(data.to_vec());
                }
            },
            |err| eprintln!("Audio capture error: {}", err),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to build input stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Failed to start stream: {}", e);
            return;
        }

        println!("Audio capture started. Say the wake word!");
        println!("Press Ctrl+C to exit.");
        println!();

        // Keep stream alive
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    // Audio processing
    let mut audio_buffer: Vec<f32> = Vec::new();
    let chunk_size = (sample_rate as usize) / 10; // 100ms chunks

    loop {
        match rx.recv() {
            Ok(samples) => {
                audio_buffer.extend_from_slice(&samples);

                // Process when we have enough samples
                while audio_buffer.len() >= chunk_size {
                    let chunk: Vec<f32> = audio_buffer.drain(..chunk_size).collect();

                    if let Some(event) = detector.process(&chunk, sample_rate as i32) {
                        println!();
                        println!("========================================");
                        println!(" WAKE WORD DETECTED: {} ", event.keyword);
                        println!("========================================");
                        println!();
                    }
                }
            }
            Err(_) => {
                eprintln!("Audio channel closed");
                break;
            }
        }
    }

    Ok(())
}
