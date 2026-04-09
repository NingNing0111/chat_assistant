//! Audio playback using rodio

use rodio::{OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer};

/// Audio player for TTS output
pub struct AudioPlayer {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Sink,
}

impl AudioPlayer {
    /// Create a new AudioPlayer
    pub fn new() -> anyhow::Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        Ok(Self { _stream: stream, _stream_handle: stream_handle, sink })
    }

    /// Play audio samples (f32 mono at 24kHz)
    pub fn play(&self, samples: &[f32]) -> anyhow::Result<()> {
        if samples.is_empty() { return Ok(()); }
        let source = SamplesBuffer::new(1, 24000, samples);
        self.sink.append(source);
        Ok(())
    }

    /// Play audio from a file
    pub fn play_file(&self, path: &str) -> anyhow::Result<()> {
        let file = std::fs::File::open(path)?;
        let source = rodio::Decoder::new(std::io::BufReader::new(file))?;
        self.sink.append(source);
        Ok(())
    }

    /// Stop current playback
    pub fn stop(&self) { self.sink.stop(); }

    /// Check if playing
    pub fn is_playing(&self) -> bool { !self.sink.is_paused() && !self.sink.empty() }

    /// Set volume (0.0 to 1.0)
    pub fn set_volume(&self, volume: f32) { self.sink.set_volume(volume.clamp(0.0, 1.0)); }

    /// Clear queue
    pub fn clear_queue(&self) { self.sink.clear(); }

    /// Wait for playback to finish
    pub fn wait(&self) {
        self.sink.sleep_until_end();
    }
}

impl Default for AudioPlayer {
    fn default() -> Self { Self::new().expect("Failed to create AudioPlayer") }
}
