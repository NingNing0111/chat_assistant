#![allow(unused, clippy::arc_with_non_send_sync)]

mod app;
mod asr;
mod audio;
mod config;
mod llm;
mod memory;
mod model_manager;
mod session;
mod tools;
mod tts;
mod wakeword;

use app::{App, AppState};
use audio::{AudioCapture, AudioPlayer};
use config::AppConfig;
use llm::ResponseSegmenter;
use model_manager::ModelManager;
use rig::client::{CompletionClient, ProviderClient};
use rig::providers::openai::Client;
use rig::tool::ToolDyn;
use rig::completion::Prompt;
use tools::builtin::{Calculate, GetTime, GetWeather, SetReminder};
use tools::McpToolLoader;
use std::sync::mpsc;

async fn collect_tools(config: &AppConfig) -> Vec<Box<dyn ToolDyn>> {
    let mut all_tools: Vec<Box<dyn ToolDyn>> = Vec::new();

    if config.tools.enable_builtin {
        all_tools.push(Box::new(GetTime));
        all_tools.push(Box::new(GetWeather));
        all_tools.push(Box::new(Calculate));
        all_tools.push(Box::new(SetReminder));
    }

    if let Some(ref mcp_path) = config.tools.mcp_config {
        match McpToolLoader::new(mcp_path).load_tools().await {
            Ok(mcp_tools) => {
                println!("Loaded {} MCP tools", mcp_tools.len());
                all_tools.extend(mcp_tools);
            }
            Err(e) => {
                println!("Warning: Failed to load MCP tools: {}", e);
            }
        }
    }

    all_tools
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let config = AppConfig::load()?;

    println!("Chat Assistant v0.1.0");
    println!("===================");
    println!("LLM: {} / {}", config.llm.provider, config.llm.model);
    println!("Wake word: {}", config.wakeword.wake_word);
    println!("ASR chunk size: {}", config.asr.chunk_size);

    // 检查并自动下载模型
    let model_manager = ModelManager::new();
    let model_status = model_manager.check_models();
    if !model_status.is_all_complete() {
        println!();
        model_manager.download_missing(&model_status).await?;
    }

    let app = App::new(config.clone()).await?;

    app.init_tts().await?;
    println!("TTS initialized");

    app.init_asr().await?;
    println!("ASR initialized");

    let continuous_mode = config.app.continuous_mode;
    if !continuous_mode {
        app.init_wakeword().await?;
        println!("WakeWord detector initialized");
    }

    let openai_client = if let Some(ref api_key) = config.llm.api_key {
        Client::builder()
            .api_key(api_key)
            .base_url(config.llm.base_url.as_deref().unwrap_or("https://api.openai.com/v1"))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build client: {}", e))?
    } else {
        Client::from_env()
    };

    let preamble = ResponseSegmenter::system_prompt();
    let tools = collect_tools(&config).await;
    println!("Total tools loaded: {}", tools.len());

    let agent = openai_client
        .agent(config.llm.model.as_str())
        .preamble(preamble)
        .tools(tools)
        .build();

    // Playback thread
    let (playback_tx, playback_rx) = mpsc::channel::<Vec<f32>>();
    std::thread::spawn(move || {
        let player = AudioPlayer::new().expect("Failed to create player");
        while let Ok(audio) = playback_rx.recv() {
            player.play(&audio[..]).ok();
        }
    });

    // Audio capture
    let (_capture, audio_rx) = AudioCapture::new(config.asr.sample_rate as u32)?;

    println!("\n[Main] Starting voice interaction loop...");
    println!("[Main] Press Ctrl+C to exit\n");

    let sample_rate = config.asr.sample_rate;
    let chunk_size = config.asr.chunk_size;

    let mut wakeword_buffer: Vec<f32> = Vec::with_capacity(sample_rate as usize);

    loop {
        let samples = match audio_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(s) => s,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("[Main] Audio channel closed");
                break;
            }
        };

        wakeword_buffer.extend_from_slice(&samples);

        while wakeword_buffer.len() >= chunk_size {
            let chunk: Vec<f32> = wakeword_buffer.drain(..chunk_size).collect();
            let state = app.get_state().await;

            match state {
                AppState::Idle | AppState::Speaking => {
                    if !continuous_mode {
                        let ww_guard = app.wakeword().await;
                        if let Some(ref ww) = *ww_guard {
                            if let Some(event) = ww.process(&chunk, sample_rate) {
                                println!("[Main] Wake word detected: {}", event.keyword);
                                app.on_wake_word().await;
                            }
                        }
                    } else if state == AppState::Idle {
                        app.on_wake_word().await;
                    }
                }
                AppState::Listening => {
                    if let Some(text) = app.process_audio(&chunk).await {
                        if !text.is_empty() {
                            println!("[User] {}", text);
                            app.set_state(AppState::Processing).await;

                            match agent.prompt(&text).await {
                                Ok(response) => {
                                    println!("[Assistant] {}", response);
                                    
                                    let tts_guard = app.tts().await;
                                    if let Some(ref tts) = *tts_guard {
                                        if let Ok(audio_data) = tts.synthesize(&response) {
                                            drop(tts_guard);
                                            playback_tx.send(audio_data).ok();
                                        }
                                    }
                                    app.set_state(AppState::Speaking).await;
                                }
                                Err(e) => {
                                    eprintln!("[Error] LLM failed: {}", e);
                                    app.set_state(AppState::Idle).await;
                                }
                            }
                        }
                    }
                }
                AppState::Processing => {
                    let mut combined = chunk.clone();
                    combined.extend_from_slice(&wakeword_buffer);
                    wakeword_buffer = combined;
                    wakeword_buffer.extend(&chunk);
                    break;
                }
            }
        }

        if wakeword_buffer.len() > sample_rate as usize * 2 {
            wakeword_buffer = wakeword_buffer.into_iter().skip(sample_rate as usize).collect();
        }
    }

    Ok(())
}
