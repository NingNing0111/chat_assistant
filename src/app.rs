use crate::asr::recognizer::AsrEvent;
use crate::asr::AsrRecognizer;
use crate::config::AppConfig;
use crate::tts::MiniMaxTtsHandler;
use crate::wakeword::WakeWordDetector;
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
    #[allow(dead_code)]
    #[allow(clippy::arc_with_non_send_sync)]
    tts: Arc<RwLock<Option<MiniMaxTtsHandler>>>,
    #[allow(dead_code)]
    #[allow(clippy::arc_with_non_send_sync)]
    asr: Arc<RwLock<Option<AsrRecognizer>>>,
    #[allow(dead_code)]
    #[allow(clippy::arc_with_non_send_sync)]
    wakeword: Arc<RwLock<Option<WakeWordDetector>>>,
}

impl App {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            state: Arc::new(RwLock::new(AppState::Idle)),
            tts: Arc::new(RwLock::new(None)),
            asr: Arc::new(RwLock::new(None)),
            wakeword: Arc::new(RwLock::new(None)),
        })
    }

    /// Initialize TTS
    pub async fn init_tts(&self) -> anyhow::Result<()> {
        let api_key = self.config.tts.api_key.clone()
            .ok_or_else(|| anyhow::anyhow!("MINIMAX_API_KEY not set"))?;

        let synthesizer = MiniMaxTtsHandler::new(
            api_key,
            self.config.tts.model.clone(),
            self.config.tts.voice_id.clone(),
            self.config.tts.speed,
            self.config.tts.sample_rate,
        );

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
    pub async fn process_audio(&self, _samples: &[f32]) -> Option<String> {
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
    pub async fn tts(&self) -> tokio::sync::RwLockReadGuard<'_, Option<MiniMaxTtsHandler>> {
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
    #[allow(dead_code)]
    pub async fn run(&self) -> anyhow::Result<()> {
        println!("[App] Starting chat assistant...");

        // Main loop would integrate with audio capture here
        // For now, just keep the app alive
        tokio::signal::ctrl_c().await?;

        Ok(())
    }
}
