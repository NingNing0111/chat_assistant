use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration loaded from environment and config files
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// LLM provider configuration
    pub llm: LlmConfig,
    /// Audio/ASR model paths
    pub asr: AsrConfig,
    /// Wake word model configuration
    pub wakeword: WakeWordConfig,
    /// TTS model configuration
    pub tts: TtsConfig,
    /// Tool/MCP configuration
    pub tools: ToolsConfig,
    /// Session configuration
    pub session: SessionConfig,
    /// Application-level settings
    pub app: AppSettings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    /// Provider name (openai, anthropic, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// API key (or set OPENAI_API_KEY env var)
    pub api_key: Option<String>,
    /// Base URL for OpenAI-compatible APIs
    pub base_url: Option<String>,
    /// Max conversation history turns
    pub max_history: usize,
    /// Default temperature
    pub temperature: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AsrConfig {
    /// Path to encoder.onnx
    pub encoder: PathBuf,
    /// Path to decoder.onnx
    pub decoder: PathBuf,
    /// Path to joiner.onnx
    pub joiner: PathBuf,
    /// Path to tokens.txt
    pub tokens: PathBuf,
    /// Provider (cpu, cuda, etc.)
    pub provider: String,
    /// Chunk size for streaming
    pub chunk_size: usize,
    /// Sample rate
    pub sample_rate: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WakeWordConfig {
    /// Path to encoder.onnx
    pub encoder: PathBuf,
    /// Path to decoder.onnx
    pub decoder: PathBuf,
    /// Path to joiner.onnx
    pub joiner: PathBuf,
    /// Path to tokens.txt
    pub tokens: PathBuf,
    /// Path to keywords file
    pub keywords: PathBuf,
    /// Provider (cpu, cuda, etc.)
    pub provider: String,
    /// Detection threshold
    pub threshold: f32,
    /// Wake word name
    pub wake_word: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    /// TTS model type (kokoro, etc.)
    pub model_type: String,
    /// Path to TTS model
    pub model: PathBuf,
    /// Path to voices file (for Kokoro)
    pub voices: Option<PathBuf>,
    /// Path to tokens file (for Kokoro)
    pub tokens: Option<PathBuf>,
    /// Path to espeak-ng data dir (for Kokoro)
    pub data_dir: Option<PathBuf>,
    /// Speech rate (0.5 - 2.0)
    pub speed: f32,
    /// Speaker ID
    pub speaker_id: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsConfig {
    /// Path to MCP tools JSON config
    pub mcp_config: Option<PathBuf>,
    /// Enable builtin tools (time, weather)
    pub enable_builtin: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    /// Max history messages to keep in context
    pub max_history: usize,
    /// Session storage path
    pub storage_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    /// Application name
    pub name: String,
    /// Default persona
    pub default_persona: String,
    /// Enable continuous conversation (no wake word needed)
    pub continuous_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                provider: "openai".into(),
                model: "gpt-4o".into(),
                api_key: None,
                base_url: None,
                max_history: 20,
                temperature: 0.8,
            },
            asr: AsrConfig {
                encoder: PathBuf::from("./models/asr/encoder.onnx"),
                decoder: PathBuf::from("./models/asr/decoder.onnx"),
                joiner: PathBuf::from("./models/asr/joiner.onnx"),
                tokens: PathBuf::from("./models/asr/tokens.txt"),
                provider: "cpu".into(),
                chunk_size: 3200,
                sample_rate: 16000,
            },
            wakeword: WakeWordConfig {
                encoder: PathBuf::from("./models/wakeword/encoder.onnx"),
                decoder: PathBuf::from("./models/wakeword/decoder.onnx"),
                joiner: PathBuf::from("./models/wakeword/joiner.onnx"),
                tokens: PathBuf::from("./models/wakeword/tokens.txt"),
                keywords: PathBuf::from("./models/wakeword/keywords.txt"),
                provider: "cpu".into(),
                threshold: 0.5,
                wake_word: "玉米糊".into(),
            },
            tts: TtsConfig {
                model_type: "kokoro".into(),
                model: PathBuf::from("./models/kokoro/model.onnx"),
                voices: Some(PathBuf::from("./models/kokoro/voices.bin")),
                tokens: Some(PathBuf::from("./models/kokoro/tokens.txt")),
                data_dir: Some(PathBuf::from("./models/kokoro/espeak-ng-data")),
                speed: 1.0,
                speaker_id: 0,
            },
            tools: ToolsConfig {
                mcp_config: None,
                enable_builtin: true,
            },
            session: SessionConfig {
                max_history: 20,
                storage_path: PathBuf::from("./data/sessions"),
            },
            app: AppSettings {
                name: "Chat Assistant".into(),
                default_persona: "assistant".into(),
                continuous_mode: false,
            },
        }
    }
}

impl AppConfig {
    /// Load configuration from environment and config files
    pub fn load() -> anyhow::Result<Self> {
        // Load from .env file if present
        dotenvy::dotenv().ok();

        // Start with defaults
        let mut config = AppConfig::default();

        // Override from environment variables
        if let Ok(provider) = std::env::var("LLM_PROVIDER") {
            config.llm.provider = provider;
        }
        if let Ok(model) = std::env::var("LLM_MODEL") {
            config.llm.model = model;
        }
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            config.llm.api_key = Some(api_key);
        }
        if let Ok(base_url) = std::env::var("LLM_BASE_URL") {
            config.llm.base_url = Some(base_url);
        }
        if let Ok(wake_word) = std::env::var("WAKE_WORD") {
            config.wakeword.wake_word = wake_word;
        }
        if let Ok(mcp_config) = std::env::var("MCP_CONFIG") {
            config.tools.mcp_config = Some(PathBuf::from(mcp_config));
        }

        Ok(config)
    }
}
