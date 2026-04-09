use crate::config::AppConfig;
use crate::memory::MemoryStore;
use crate::session::SessionManager;
use crate::tts::TtsSynthesizer;
use crate::wakeword::WakeWordDetector;
use crate::asr::recognizer::AsrEvent;
use crate::asr::AsrRecognizer;
use crate::audio::{AudioCapture, AudioPlayer};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Idle,
    Listening,
    Processing,
    Speaking,
}

/// Main application orchestrating all components
pub struct App {
    config: AppConfig,
    state: Arc<RwLock<AppState>>,
    session_manager: SessionManager,
    memory_store: MemoryStore,
    tts: Arc<RwLock<Option<TtsSynthesizer>>>,
    asr: Arc<RwLock<Option<AsrRecognizer>>>,
    wakeword: Arc<RwLock<Option<WakeWordDetector>>>,
    continuous_mode: Arc<RwLock<bool>>,
}

impl App {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let session_manager = SessionManager::new(config.session.max_history);
        let memory_store = MemoryStore::new();

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(AppState::Idle)),
            session_manager,
            memory_store,
            tts: Arc::new(RwLock::new(None)),
            asr: Arc::new(RwLock::new(None)),
            wakeword: Arc::new(RwLock::new(None)),
            continuous_mode: Arc::new(RwLock::new(false)),
        })
    }

    /// Initialize TTS
    pub async fn init_tts(&self) -> anyhow::Result<()> {
        let synthesizer = TtsSynthesizer::new_kokoro(
            &self.config.tts.model,
            self.config.tts.voices.as_deref(),
            self.config.tts.tokens.as_deref(),
            self.config.tts.data_dir.as_deref(),
            self.config.tts.speed,
            self.config.tts.speaker_id,
        )?;

        *self.tts.write().await = Some(synthesizer);
        Ok(())
    }

    /// Initialize ASR
    pub async fn init_asr(&self) -> anyhow::Result<()> {
        let recognizer = AsrRecognizer::from_config(
            self.config.asr.encoder.to_str().unwrap_or(""),
            self.config.asr.decoder.to_str().unwrap_or(""),
            self.config.asr.joiner.to_str().unwrap_or(""),
            self.config.asr.tokens.to_str().unwrap_or(""),
            &self.config.asr.provider,
            self.config.asr.chunk_size,
            self.config.asr.sample_rate,
        )?;

        *self.asr.write().await = Some(recognizer);
        Ok(())
    }

    /// Initialize WakeWord detection
    pub async fn init_wakeword(&self) -> anyhow::Result<()> {
        let detector = WakeWordDetector::new(
            &self.config.wakeword.encoder,
            &self.config.wakeword.decoder,
            &self.config.wakeword.joiner,
            &self.config.wakeword.tokens,
            &self.config.wakeword.keywords,
            &self.config.wakeword.provider,
        )?;

        *self.wakeword.write().await = Some(detector);
        Ok(())
    }

    /// Set continuous mode (no wake word needed)
    pub async fn set_continuous_mode(&self, enabled: bool) {
        *self.continuous_mode.write().await = enabled;
    }

    /// Check if continuous mode is enabled
    pub async fn is_continuous_mode(&self) -> bool {
        *self.continuous_mode.read().await
    }

    /// Handle wake word detection
    pub async fn on_wake_word(&self) {
        let mut state = self.state.write().await;
        if *state == AppState::Idle || *state == AppState::Speaking {
            *state = AppState::Listening;
            println!("[App] Wake word detected, listening...");
        }
    }

    /// Handle barge-in (interrupt from user)
    pub async fn on_barge_in(&self) {
        if let Some(ref mut asr) = *self.asr.write().await {
            asr.reset();
        }

        let mut state = self.state.write().await;
        *state = AppState::Listening;
        println!("[App] Barge-in detected, interrupting...");
    }

    /// Process audio input through ASR
    pub async fn process_audio(&self, samples: &[f32]) -> Option<String> {
        let state = self.state.read().await;
        if *state != AppState::Listening {
            return None;
        }
        drop(state);

        let mut asr = self.asr.write().await;
        if let Some(ref mut recognizer) = *asr {
            let result = recognizer.process();
            if result.is_some() {
                *self.state.write().await = AppState::Processing;
            }
            // Convert AsrEvent to String
            result.map(|event| match event {
                AsrEvent::Partial(s) => s,
                AsrEvent::Final(s) => s,
            })
        } else {
            None
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> AppState {
        self.state.read().await.clone()
    }

    /// Get TTS guard
    pub async fn tts(&self) -> tokio::sync::RwLockReadGuard<'_, Option<TtsSynthesizer>> {
        self.tts.read().await
    }

    /// Get wakeword guard
    pub async fn wakeword(&self) -> tokio::sync::RwLockReadGuard<'_, Option<WakeWordDetector>> {
        self.wakeword.read().await
    }

    /// Set state
    pub async fn set_state(&self, state: AppState) {
        *self.state.write().await = state;
    }

    /// Run the main interaction loop
    pub async fn run(&self) -> anyhow::Result<()> {
        let continuous = self.is_continuous_mode().await;
        println!("[App] Starting chat assistant...");
        println!("[App] Continuous mode: {}", continuous);

        // Main loop would integrate with audio capture here
        // For now, just keep the app alive
        tokio::signal::ctrl_c().await?;

        Ok(())
    }
}