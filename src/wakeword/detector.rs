/// Wake word detection using sherpa-onnx KeywordSpotter
///
/// Based on sherpa-onnx rust-api-examples/keyword_spotter.rs

use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, Wave};
use std::path::Path;

/// Wake word detection result
#[derive(Debug, Clone)]
pub struct WakeWordEvent {
    pub keyword: String,
    pub score: f32,
}

/// Wake word detector
pub struct WakeWordDetector {
    kws: KeywordSpotter,
}

impl WakeWordDetector {
    /// Create a new WakeWordDetector from config paths
    pub fn new(
        encoder: &Path,
        decoder: &Path,
        joiner: &Path,
        tokens: &Path,
        keywords_file: &Path,
        provider: &str,
    ) -> anyhow::Result<Self> {
        let mut config = KeywordSpotterConfig::default();
        config.model_config.transducer.encoder = Some(encoder.to_string_lossy().into_owned());
        config.model_config.transducer.decoder = Some(decoder.to_string_lossy().into_owned());
        config.model_config.transducer.joiner = Some(joiner.to_string_lossy().into_owned());
        config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
        config.model_config.provider = Some(provider.to_string());
        config.model_config.num_threads = 1;
        config.model_config.debug = false;
        config.keywords_file = Some(keywords_file.to_string_lossy().into_owned());

        let kws = KeywordSpotter::create(&config)
            .ok_or_else(|| anyhow::anyhow!("Failed to create KeywordSpotter"))?;

        Ok(Self { kws })
    }

    /// Process audio samples and check for wake word
    ///
    /// Returns WakeWordEvent if detected, None otherwise.
    /// Audio samples should be at 16kHz mono f32.
    pub fn process(&self, samples: &[f32], sample_rate: i32) -> Option<WakeWordEvent> {
        let stream = self.kws.create_stream();

        // Add tail padding (500ms)
        let tail_padding = vec![0.0f32; (sample_rate / 2) as usize];

        // Process audio
        let stream = stream;
        stream.accept_waveform(sample_rate, samples);
        stream.accept_waveform(sample_rate, &tail_padding);
        stream.input_finished();

        // Decode
        while self.kws.is_ready(&stream) {
            self.kws.decode(&stream);
        }

        // Get result
        if let Some(result) = self.kws.get_result(&stream) {
            if !result.keyword.is_empty() {
                return Some(WakeWordEvent {
                    keyword: result.keyword,
                    score: 0.0, // score not available in result
                });
            }
        }

        None
    }

    /// Detect from WAV file
    pub fn detect_from_wav(&self, wav_path: &Path) -> anyhow::Result<Option<WakeWordEvent>> {
        let wave = Wave::read(&wav_path.to_string_lossy())
            .ok_or_else(|| anyhow::anyhow!("Failed to read WAV file: {}", wav_path.display()))?;

        let sample_rate = wave.sample_rate() as i32;
        let samples = wave.samples();

        Ok(self.process(samples, sample_rate))
    }

    /// Get the KeywordSpotter instance for advanced usage
    pub fn inner(&self) -> &KeywordSpotter {
        &self.kws
    }
}
