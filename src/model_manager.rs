//! Model Manager - 自动下载和管理模型文件
//!
//! 首次启动时自动检测并下载缺失的模型文件

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

const WAKEWORD_MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-wenetspeech-3.3M-2024-01-01-mobile.tar.bz2";
const ASR_MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2";
const TTS_MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_0.tar.bz2";

const WAKEWORD_FILES: &[&str] = &[
    "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
    "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
    "joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
    "tokens.txt",
    "keywords.txt",
];

const ASR_FILES: &[&str] = &[
    "encoder-epoch-99-avg-1.int8.onnx",
    "decoder-epoch-99-avg-1.onnx",
    "joiner-epoch-99-avg-1.int8.onnx",
    "tokens.txt",
];

const TTS_FILES: &[&str] = &[
    "model.onnx",
    "voices.bin",
    "tokens.txt",
];

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            models_dir: PathBuf::from("./models"),
        }
    }

    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// 检查所有模型是否存在
    pub fn check_models(&self) -> ModelStatus {
        let wakeword_ok = self.check_dir("wakeword", WAKEWORD_FILES);
        let asr_ok = self.check_dir("asr", ASR_FILES);
        let tts_ok = self.check_dir("kokoro", TTS_FILES);

        ModelStatus {
            wakeword: wakeword_ok,
            asr: asr_ok,
            tts: tts_ok,
        }
    }

    fn check_dir(&self, dir: &str, files: &[&str]) -> bool {
        let dir_path = self.models_dir.join(dir);
        if !dir_path.exists() {
            return false;
        }
        files.iter().all(|f| dir_path.join(f).exists())
    }

    /// 下载缺失的模型
    pub async fn download_missing(&self, status: &ModelStatus) -> Result<()> {
        println!("\n📦 Model Check & Download");
        println!("{}", "-".repeat(50));

        if status.is_all_complete() {
            println!("✅ All models are present!");
            return Ok(());
        }

        println!("⚠️  Missing models detected, downloading...");
        println!("   Missing: {}\n", status.missing().join(", "));

        // 下载 ASR 模型 (最大)
        if !status.asr {
            println!("↓ Downloading ASR model (中英文流式语音识别, ~487MB)...");
            self.download_and_extract(ASR_MODEL_URL, "asr").await?;
        }

        // 下载 TTS 模型
        if !status.tts {
            println!("\n↓ Downloading TTS model (Kokoro 中英文语音合成, ~333MB)...");
            self.download_and_extract(TTS_MODEL_URL, "kokoro").await?;
        }

        // 下载 WakeWord 模型
        if !status.wakeword {
            println!("\n↓ Downloading WakeWord model (唤醒词检测, ~14MB)...");
            self.download_and_extract(WAKEWORD_MODEL_URL, "wakeword").await?;
            self.setup_wakeword_keywords().await?;
        }

        // 创建符号链接
        self.create_symlinks().await?;

        println!("\n✅ All models downloaded successfully!");
        Ok(())
    }

    async fn download_and_extract(&self, url: &str, dir: &str) -> Result<()> {
        let dir_path = self.models_dir.join(dir);
        std::fs::create_dir_all(&dir_path)
            .context(format!("Failed to create directory: {:?}", dir_path))?;

        let archive_name = url.split('/').last().unwrap();
        let archive_path = dir_path.join(archive_name);

        // 下载文件
        println!("  Downloading {}...", archive_name);

        let client = reqwest::Client::new();
        let response = client.get(url)
            .send()
            .await
            .context("Failed to download model")?;

        let total_size = response.content_length().unwrap_or(0);
        println!("  Total size: {:.1} MB", total_size as f64 / 1024.0 / 1024.0);

        let bytes = response.bytes().await
            .context("Failed to read response body")?;

        println!("  Saving...");
        std::fs::write(&archive_path, &bytes)
            .context("Failed to save archive")?;

        // 解压
        println!("  Extracting...");
        let process = std::process::Command::new("tar")
            .args(["xjf", archive_path.to_str().unwrap()])
            .current_dir(&dir_path)
            .output()
            .context("Failed to extract archive")?;

        if !process.status.success() {
            anyhow::bail!("Failed to extract archive");
        }

        // 移动文件到根目录
        let entries = std::fs::read_dir(&dir_path)?;
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let src = entry.path();
            let dst = dir_path.join(&name);

            // 跳过已经是文件的情况
            if dst.exists() && dst.is_file() {
                std::fs::remove_file(&src).ok();
                continue;
            }

            // 移动目录内容
            if src.is_dir() && !name.to_string_lossy().starts_with('.') {
                if let Ok(items) = std::fs::read_dir(&src) {
                    for item in items.flatten() {
                        let item_name = item.file_name();
                        std::fs::rename(item.path(), dst.join(&item_name)).ok();
                    }
                }
                std::fs::remove_dir(&src).ok();
            }
        }

        // 删除压缩包
        std::fs::remove_file(&archive_path).ok();

        Ok(())
    }

    async fn setup_wakeword_keywords(&self) -> Result<()> {
        let keywords_path = self.models_dir.join("wakeword").join("keywords.txt");
        let raw_path = self.models_dir.join("wakeword").join("keywords_raw.txt");

        // 如果有原始关键词文件，复制一份
        if raw_path.exists() {
            std::fs::copy(&raw_path, &keywords_path)?;
        }

        Ok(())
    }

    async fn create_symlinks(&self) -> Result<()> {
        // WakeWord 符号链接
        let ww_dir = self.models_dir.join("wakeword");
        if !ww_dir.join("encoder.onnx").exists() {
            let encoder = ww_dir.join("encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx");
            if encoder.exists() {
                std::os::unix::fs::symlink(&encoder, ww_dir.join("encoder.onnx"))?;
            }
        }
        if !ww_dir.join("decoder.onnx").exists() {
            let decoder = ww_dir.join("decoder-epoch-12-avg-2-chunk-16-left-64.onnx");
            if decoder.exists() {
                std::os::unix::fs::symlink(&decoder, ww_dir.join("decoder.onnx"))?;
            }
        }
        if !ww_dir.join("joiner.onnx").exists() {
            let joiner = ww_dir.join("joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx");
            if joiner.exists() {
                std::os::unix::fs::symlink(&joiner, ww_dir.join("joiner.onnx"))?;
            }
        }

        // ASR 符号链接
        let asr_dir = self.models_dir.join("asr");
        if !asr_dir.join("encoder.onnx").exists() {
            let encoder = asr_dir.join("encoder-epoch-99-avg-1.int8.onnx");
            if encoder.exists() {
                std::os::unix::fs::symlink(&encoder, asr_dir.join("encoder.onnx"))?;
            }
        }
        if !asr_dir.join("decoder.onnx").exists() {
            let decoder = asr_dir.join("decoder-epoch-99-avg-1.onnx");
            if decoder.exists() {
                std::os::unix::fs::symlink(&decoder, asr_dir.join("decoder.onnx"))?;
            }
        }
        if !asr_dir.join("joiner.onnx").exists() {
            let joiner = asr_dir.join("joiner-epoch-99-avg-1.int8.onnx");
            if joiner.exists() {
                std::os::unix::fs::symlink(&joiner, asr_dir.join("joiner.onnx"))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub wakeword: bool,
    pub asr: bool,
    pub tts: bool,
}

impl ModelStatus {
    pub fn is_all_complete(&self) -> bool {
        self.wakeword && self.asr && self.tts
    }

    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.wakeword { missing.push("WakeWord"); }
        if !self.asr { missing.push("ASR"); }
        if !self.tts { missing.push("TTS"); }
        missing
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}
