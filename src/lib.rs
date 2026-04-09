//! Chat Assistant Library
//!
//! A voice-interactive AI assistant using:
//! - rig for LLM and tool calling
//! - sherpa-onnx for ASR, TTS, and wake word detection
//! - cpal for audio capture
//! - rodio for audio playback

pub mod app;
pub mod asr;
pub mod audio;
pub mod config;
pub mod llm;
pub mod memory;
pub mod model_manager;
pub mod session;
pub mod tools;
pub mod tts;
pub mod wakeword;
