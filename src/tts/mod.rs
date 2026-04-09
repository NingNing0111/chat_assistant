pub mod minimax;
pub use minimax::MiniMaxTtsHandler;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Text buffer that accumulates characters and extracts segments on stop markers
pub struct TextBuffer {
    buffer: String,
    stop_marker: String,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            stop_marker: "<stop/>".to_string(),
        }
    }

    /// Push a chunk of text into the buffer
    /// Returns Some(segment) if a stop marker was found, None otherwise
    pub fn push(&mut self, chunk: &str) -> Option<String> {
        self.buffer.push_str(chunk);

        // Check for stop marker
        if let Some(pos) = self.buffer.find(&self.stop_marker) {
            let segment = self.buffer[..pos].trim().to_string();
            let remaining = self.buffer[pos + self.stop_marker.len()..].to_string();
            self.buffer = remaining;

            if !segment.is_empty() {
                return Some(segment);
            }
        }
        None
    }

    /// Extract all remaining text (flush the buffer)
    pub fn extract_all(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }
        let remaining = self.buffer.trim().to_string();
        self.buffer.clear();
        if remaining.is_empty() {
            None
        } else {
            Some(remaining)
        }
    }

    /// Get current buffer length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// TTS Queue that handles async synthesis
/// Note: AudioPlayer must be used on the main thread, so playback happens synchronously
pub struct TtsQueue {
    tts: Arc<MiniMaxTtsHandler>,
    tx: mpsc::Sender<TtsJob>,
}

struct TtsJob {
    text: String,
}

/// Start the TTS queue worker and return a handle
pub fn start_tts_queue(
    tts: MiniMaxTtsHandler,
) -> (TtsQueue, mpsc::Receiver<Vec<f32>>) {
    let tts = Arc::new(tts);
    let tts_for_queue = tts.clone();
    let (job_tx, mut job_rx) = mpsc::channel::<TtsJob>(32);
    let (sample_tx, sample_rx) = mpsc::channel::<Vec<f32>>(32);

    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(job) = job_rx.recv() => {
                    let tts_clone = tts.clone();
                    match tts_clone.synthesize(&job.text).await {
                        Ok(samples) => {
                            if !samples.is_empty() {
                                if sample_tx.send(samples).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("TTS synthesis error: {}", e);
                        }
                    }
                }
                else => break,
            }
        }
    });

    let queue = TtsQueue { tts: tts_for_queue, tx: job_tx };
    (queue, sample_rx)
}

impl TtsQueue {
    /// Enqueue text for TTS synthesis (non-blocking)
    pub async fn enqueue(&self, text: String) -> Result<()> {
        self.tx
            .send(TtsJob { text })
            .await
            .map_err(|_| anyhow::anyhow!("Failed to enqueue TTS"))?;
        Ok(())
    }
}

// Re-export AudioPlayer for convenience
pub use crate::audio::AudioPlayer;
