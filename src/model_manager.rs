//! Model Manager - 自动下载和管理模型文件
//!
//! 首次启动时自动检测并下载缺失的模型文件

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const WAKEWORD_MODEL_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/kws-models/sherpa-onnx-kws-zipformer-wenetspeech-3.3M-2024-01-01-mobile.tar.bz2";
const ASR_MODEL_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-bilingual-zh-en-2023-02-20.tar.bz2";

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

        ModelStatus {
            wakeword: wakeword_ok,
            asr: asr_ok,
        }
    }

    /// 清理嵌套目录残留（tar 解压可能产生多余层级）
    pub fn cleanup_nested_dirs(&self) {
        for dir in ["wakeword", "asr"] {
            let dir_path = self.models_dir.join(dir);
            if dir_path.exists() {
                self.move_extracted_files(&dir_path).ok();
            }
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
        println!("\nModel Check & Download");
        println!("{}", "-".repeat(50));

        if status.is_all_complete() {
            println!("All models are present!");
            return Ok(());
        }

        println!("Missing models detected, downloading...");
        println!("Missing: {}\n", status.missing().join(", "));

        // 下载 ASR 模型 (最大)
        if !status.asr {
            println!("Downloading ASR model (Chinese/English ASR, ~487MB)...");
            self.download_and_extract(ASR_MODEL_URL, "asr").await?;
        }

        // 下载 WakeWord 模型
        if !status.wakeword {
            println!("\nDownloading WakeWord model (~14MB)...");
            self.download_and_extract(WAKEWORD_MODEL_URL, "wakeword").await?;
            self.setup_wakeword_keywords().await?;
        }

        // 创建符号链接或复制文件
        self.create_shortcuts().await?;

        println!("\nAll models downloaded successfully!");
        Ok(())
    }

    async fn download_and_extract(&self, url: &str, dir: &str) -> Result<()> {
        let dir_path = self.models_dir.join(dir);
        std::fs::create_dir_all(&dir_path)
            .context(format!("Failed to create directory: {:?}", dir_path))?;

        let archive_name = url.split('/').next_back().unwrap();
        let archive_path = dir_path.join(archive_name);

        // 下载文件
        println!("  Downloading {}...", archive_name);

        let client = reqwest::Client::new();
        let response = client.get(url).send().await.context("Failed to download model")?;

        let total_size = response.content_length().unwrap_or(0);
        println!("  Total size: {:.1} MB", total_size as f64 / 1024.0 / 1024.0);

        let bytes = response.bytes().await.context("Failed to read response body")?;

        println!("  Saving...");
        std::fs::write(&archive_path, &bytes).context("Failed to save archive")?;

        // 解压
        println!("  Extracting...");
        self.extract_archive(&archive_path, &dir_path)?;

        // 移动文件到根目录（展平嵌套目录）
        self.move_extracted_files(&dir_path)?;

        // 再次清理残留嵌套（部分 tar 可能多层嵌套）
        self.move_extracted_files(&dir_path)?;

        // 删除压缩包
        std::fs::remove_file(&archive_path).ok();

        // 验证文件存在
        let files_ok = match dir {
            "wakeword" => self.check_dir("wakeword", WAKEWORD_FILES),
            "asr" => self.check_dir("asr", ASR_FILES),
            _ => true,
        };
        if !files_ok {
            anyhow::bail!("Model extraction failed: {} dir is missing required files", dir);
        }

        Ok(())
    }

    fn extract_archive(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = std::fs::File::open(archive_path).context("Failed to open archive")?;
        let file = std::io::BufReader::new(file);

        // 根据扩展名选择解压方式
        if archive_path
            .extension()
            .map(|e| e == "bz2")
            .unwrap_or(false)
        {
            // 使用 bzip2 解压
            let decoder = bzip2::bufread::BzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive
                .unpack(dest_dir)
                .context("Failed to unpack tar.bz2 archive")?;
        } else if archive_path
            .extension()
            .map(|e| e == "gz")
            .unwrap_or(false)
        {
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);
            archive
                .unpack(dest_dir)
                .context("Failed to unpack tar.gz archive")?;
        } else {
            // 尝试作为纯 tar 文件
            let mut archive = tar::Archive::new(file);
            archive.unpack(dest_dir).context("Failed to unpack tar archive")?;
        }

        Ok(())
    }

    fn move_extracted_files(&self, dir_path: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir_path)?;
        for entry in entries.flatten() {
            let src = entry.path();
            let name = entry.file_name();

            // 跳过非目录（压缩包等已在调用者删除）
            if !src.is_dir() {
                continue;
            }

            // 跳过隐藏目录
            if name.to_string_lossy().starts_with('.') {
                continue;
            }

            // 移动目录内容到父目录
            if let Ok(items) = std::fs::read_dir(&src) {
                for item in items.flatten() {
                    let item_path = item.path();
                    let dst = dir_path.join(item.file_name());

                    // copy + delete（Windows 跨目录 rename 会失败）
                    if item_path.is_file() {
                        std::fs::copy(&item_path, &dst)?;
                        std::fs::remove_file(&item_path).ok();
                    } else if item_path.is_dir() {
                        Self::copy_dir_recursive(&item_path, &dst)?;
                        std::fs::remove_dir_all(&item_path).ok();
                    }
                }
            }

            // 删除空目录
            std::fs::remove_dir(&src).ok();
        }

        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
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

    async fn create_shortcuts(&self) -> Result<()> {
        // WakeWord 快捷方式
        let ww_dir = self.models_dir.join("wakeword");
        self.create_shortcut_if_needed(
            &ww_dir,
            "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
            "encoder.onnx",
        )?;
        self.create_shortcut_if_needed(
            &ww_dir,
            "decoder-epoch-12-avg-2-chunk-16-left-64.onnx",
            "decoder.onnx",
        )?;
        self.create_shortcut_if_needed(
            &ww_dir,
            "joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx",
            "joiner.onnx",
        )?;

        // ASR 快捷方式
        let asr_dir = self.models_dir.join("asr");
        self.create_shortcut_if_needed(
            &asr_dir,
            "encoder-epoch-99-avg-1.int8.onnx",
            "encoder.onnx",
        )?;
        self.create_shortcut_if_needed(
            &asr_dir,
            "decoder-epoch-99-avg-1.onnx",
            "decoder.onnx",
        )?;
        self.create_shortcut_if_needed(
            &asr_dir,
            "joiner-epoch-99-avg-1.int8.onnx",
            "joiner.onnx",
        )?;

        Ok(())
    }

    fn create_shortcut_if_needed(
        &self,
        dir: &Path,
        source_name: &str,
        link_name: &str,
    ) -> Result<()> {
        let link_path = dir.join(link_name);

        // 如果快捷方式已存在，跳过
        if link_path.exists() {
            return Ok(());
        }

        let source_path = dir.join(source_name);

        // 如果源文件不存在，跳过
        if !source_path.exists() {
            return Ok(());
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&source_path, &link_path)?;
        }

        #[cfg(windows)]
        {
            // Windows 上复制文件作为替代
            std::fs::copy(&source_path, &link_path)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModelStatus {
    pub wakeword: bool,
    pub asr: bool,
}

impl ModelStatus {
    pub fn is_all_complete(&self) -> bool {
        self.wakeword && self.asr
    }

    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if !self.wakeword {
            missing.push("WakeWord");
        }
        if !self.asr {
            missing.push("ASR");
        }
        missing
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}
