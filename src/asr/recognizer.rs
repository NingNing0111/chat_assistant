/// Streaming ASR recognizer using sherpa-onnx OnlineRecognizer
///
/// Based on sherpa-onnx rust-api-examples/streaming_zipformer_microphone.rs

use cpal::traits::DeviceTrait;
use cpal::{SampleFormat, Stream};
use sherpa_onnx::{OnlineRecognizer, OnlineRecognizerConfig, OnlineStream};

/// ASR event types
#[derive(Debug, Clone)]
pub enum AsrEvent {
    Partial(String),
    Final(String),
}

/// Streaming ASR recognizer
pub struct AsrRecognizer {
    recognizer: OnlineRecognizer,
    stream: OnlineStream,
    sample_rate: i32,
    chunk_size: usize,
}

impl AsrRecognizer {
    /// Create from config
    pub fn from_config(
        encoder: &str,
        decoder: &str,
        joiner: &str,
        tokens: &str,
        provider: &str,
        chunk_size: usize,
        sample_rate: i32,
    ) -> anyhow::Result<Self> {
        let mut config = OnlineRecognizerConfig::default();
        config.model_config.transducer.encoder = Some(encoder.to_string());
        config.model_config.transducer.decoder = Some(decoder.to_string());
        config.model_config.transducer.joiner = Some(joiner.to_string());
        config.model_config.tokens = Some(tokens.to_string());
        config.model_config.provider = Some(provider.to_string());
        config.model_config.num_threads = 1;
        config.model_config.debug = false;
        config.enable_endpoint = true;
        config.decoding_method = Some("greedy_search".to_string());

        let recognizer = OnlineRecognizer::create(&config)
            .expect("Failed to create OnlineRecognizer");

        let stream = recognizer.create_stream();

        Ok(Self {
            recognizer,
            stream,
            sample_rate,
            chunk_size,
        })
    }

    /// Feed audio samples
    pub fn accept_waveform(&mut self, samples: &[f32]) {
        self.stream.accept_waveform(self.sample_rate, samples);
    }

    /// Process and return result if ready
    pub fn process(&mut self) -> Option<AsrEvent> {
        if self.recognizer.is_ready(&self.stream) {
            self.recognizer.decode(&self.stream);

            if let Some(result) = self.recognizer.get_result(&self.stream) {
                if !result.text.is_empty() {
                    if self.recognizer.is_endpoint(&self.stream) {
                        self.recognizer.reset(&self.stream);
                        return Some(AsrEvent::Final(result.text));
                    }
                    return Some(AsrEvent::Partial(result.text));
                }
            }
        }
        None
    }

    /// Reset the recognizer state
    pub fn reset(&mut self) {
        self.recognizer.reset(&self.stream);
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Get the inner recognizer
    pub fn inner(&self) -> &OnlineRecognizer {
        &self.recognizer
    }
}

/// Build audio input stream from microphone
pub fn build_input_stream(
    device: &cpal::Device,
    _sample_rate: u32,
    channels: u16,
    on_audio: impl Fn(Vec<f32>) + Send + 'static,
) -> anyhow::Result<Stream> {
    let supported = device.default_input_config()?;
    let config = supported.config();
    let sample_format = supported.sample_format();

    let err_fn = |err| eprintln!("Audio stream error: {:?}", err);
    let on_audio = std::sync::Mutex::new(on_audio);

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                if data.is_empty() {
                    return;
                }
                let mono: Vec<f32> = if channels == 1 {
                    data.to_vec()
                } else {
                    data.chunks(channels as usize)
                        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
                        .collect()
                };
                let _ = on_audio.lock().unwrap()(mono);
            },
            err_fn,
            None,
        )?,

        SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                if data.is_empty() {
                    return;
                }
                let mono: Vec<f32> = if channels == 1 {
                    data.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
                } else {
                    data.chunks(channels as usize)
                        .map(|frame| {
                            frame.iter().map(|&s| s as f32 / i16::MAX as f32).sum::<f32>() / channels as f32
                        })
                        .collect()
                };
                let _ = on_audio.lock().unwrap()(mono);
            },
            err_fn,
            None,
        )?,

        _ => anyhow::bail!("Unsupported sample format: {:?}", sample_format),
    };

    Ok(stream)
}

/// Builder for ASR with channel-based callback
pub struct AsrRecognizerBuilder {
    encoder: String,
    decoder: String,
    joiner: String,
    tokens: String,
    provider: String,
    chunk_size: usize,
    sample_rate: i32,
}

impl AsrRecognizerBuilder {
    pub fn new() -> Self {
        Self {
            encoder: String::new(),
            decoder: String::new(),
            joiner: String::new(),
            tokens: String::new(),
            provider: "cpu".into(),
            chunk_size: 3200,
            sample_rate: 16000,
        }
    }

    pub fn encoder(mut self, path: &str) -> Self {
        self.encoder = path.into();
        self
    }

    pub fn decoder(mut self, path: &str) -> Self {
        self.decoder = path.into();
        self
    }

    pub fn joiner(mut self, path: &str) -> Self {
        self.joiner = path.into();
        self
    }

    pub fn tokens(mut self, path: &str) -> Self {
        self.tokens = path.into();
        self
    }

    pub fn provider(mut self, p: &str) -> Self {
        self.provider = p.into();
        self
    }

    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    pub fn sample_rate(mut self, sr: i32) -> Self {
        self.sample_rate = sr;
        self
    }

    /// Build recognizer without spawning thread
    pub fn build(self) -> anyhow::Result<AsrRecognizer> {
        AsrRecognizer::from_config(
            &self.encoder,
            &self.decoder,
            &self.joiner,
            &self.tokens,
            &self.provider,
            self.chunk_size,
            self.sample_rate,
        )
    }
}

impl Default for AsrRecognizerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
