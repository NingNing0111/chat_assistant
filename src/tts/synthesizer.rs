/// TTS synthesizer using sherpa-onnx Kokoro
///
/// Based on sherpa-onnx rust-api-examples/kokoro_tts_en.rs

use sherpa_onnx::{
    GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig,
};
use std::path::Path;

/// TTS synthesizer for text-to-speech
pub struct TtsSynthesizer {
    tts: OfflineTts,
    sample_rate: i32,
    num_speakers: i32,
    speed: f32,
    speaker_id: usize,
}

impl TtsSynthesizer {
    /// Create Kokoro TTS synthesizer from config
    pub fn new_kokoro(
        model_path: &Path,
        voices_path: Option<&Path>,
        tokens_path: Option<&Path>,
        data_dir: Option<&Path>,
        speed: f32,
        speaker_id: usize,
    ) -> anyhow::Result<Self> {
        let config = OfflineTtsConfig {
            model: sherpa_onnx::OfflineTtsModelConfig {
                kokoro: OfflineTtsKokoroModelConfig {
                    model: Some(model_path.to_string_lossy().into_owned()),
                    voices: voices_path.map(|p| p.to_string_lossy().into_owned()),
                    tokens: tokens_path.map(|p| p.to_string_lossy().into_owned()),
                    data_dir: data_dir.map(|p| p.to_string_lossy().into_owned()),
                    length_scale: 1.0 / speed,
                    ..Default::default()
                },
                num_threads: 2,
                debug: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let tts = OfflineTts::create(&config)
            .ok_or_else(|| anyhow::anyhow!("Failed to create OfflineTts"))?;

        let sample_rate = tts.sample_rate() as i32;
        let num_speakers = tts.num_speakers();

        Ok(Self {
            tts,
            sample_rate,
            num_speakers,
            speed,
            speaker_id,
        })
    }

    /// Synthesize text to audio samples
    pub fn synthesize(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let sid = if self.speaker_id < self.num_speakers as usize {
            self.speaker_id as i32
        } else {
            0
        };

        let gen_config = GenerationConfig {
            sid,
            speed: self.speed,
            ..Default::default()
        };

        let audio = self
            .tts
            .generate_with_config::<fn(&[f32], f32) -> bool>(text, &gen_config, None)
            .ok_or_else(|| anyhow::anyhow!("TTS generation failed"))?;

        Ok(audio.samples().to_vec())
    }

    /// Synthesize text with progress callback
    pub fn synthesize_with_callback<F>(&self, text: &str, mut on_progress: F) -> anyhow::Result<Vec<f32>>
    where
        F: FnMut(f32) + 'static,
    {
        let sid = if self.speaker_id < self.num_speakers as usize {
            self.speaker_id as i32
        } else {
            0
        };

        let gen_config = GenerationConfig {
            sid,
            speed: self.speed,
            ..Default::default()
        };

        let audio = self
            .tts
            .generate_with_config(
                text,
                &gen_config,
                Some(move |_samples: &[f32], progress: f32| -> bool {
                    on_progress(progress);
                    true
                }),
            )
            .ok_or_else(|| anyhow::anyhow!("TTS generation failed"))?;

        Ok(audio.samples().to_vec())
    }

    /// Get supported voices count
    pub fn num_speakers(&self) -> i32 {
        self.num_speakers
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }
}
