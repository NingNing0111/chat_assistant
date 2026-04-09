//! Audio capture from microphone using cpal

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

/// Audio capture from microphone, sending mono f32 samples via channel
pub struct AudioCapture {
    _stream_handle: thread::JoinHandle<()>,
}

impl AudioCapture {
    /// Create a new AudioCapture and start capturing in background thread
    pub fn new(_sample_rate: u32) -> anyhow::Result<(Self, Receiver<Vec<f32>>)> {
        let (tx, rx) = channel();

        let stream_handle = thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("No input device available");
                    return;
                }
            };

            println!("Using input device: {}", device.name().unwrap_or_default());

            let config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to get input config: {}", e);
                    return;
                }
            };

            let stream = match device.build_input_stream(
                &config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !data.is_empty() {
                        let _ = tx.send(data.to_vec());
                    }
                },
                |err| eprintln!("Audio capture error: {}", err),
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to build input stream: {}", e);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("Failed to start stream: {}", e);
                return;
            }

            // Keep stream alive by sleeping
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Ok((Self { _stream_handle: stream_handle }, rx))
    }

    /// List available input devices
    pub fn list_devices() -> anyhow::Result<Vec<String>> {
        let host = cpal::default_host();
        host.input_devices()
            .map(|devs| devs.filter_map(|d| d.name().ok()).collect())
            .map_err(|e| anyhow::anyhow!("Failed to list devices: {}", e))
    }
}
