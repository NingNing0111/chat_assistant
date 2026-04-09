//! Audio Playback Example - 测试生成的TTS音频播放
//!
//! 运行: cargo run --example audio_playback

use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

fn main() -> anyhow::Result<()> {
    let audio_file = "generated_tts.wav";

    println!("Playing: {}", audio_file);

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(audio_file)?;
    let source = Decoder::new(BufReader::new(file))?;
    sink.append(source);

    println!("Playing... (press Ctrl+C to stop)");

    // Wait for playback to finish
    sink.sleep_until_end();

    println!("Done!");
    Ok(())
}
