//! TTS Example - 测试 Kokoro TTS 语音合成
//!
//! 运行: cargo run --example tts_example

use sherpa_onnx::{GenerationConfig, OfflineTts, OfflineTtsConfig, OfflineTtsKokoroModelConfig};
use std::io::Write;
use std::time::Instant;

fn main() {
    let model_dir = "./models/kokoro";

    println!("Loading TTS model from: {}", model_dir);

    let config = OfflineTtsConfig {
        model: sherpa_onnx::OfflineTtsModelConfig {
            kokoro: OfflineTtsKokoroModelConfig {
                model: Some(format!("{}/model.onnx", model_dir).into()),
                voices: Some(format!("{}/voices.bin", model_dir).into()),
                tokens: Some(format!("{}/tokens.txt", model_dir).into()),
                data_dir: Some(format!("{}/espeak-ng-data", model_dir).into()),
                dict_dir: Some(format!("{}/dict", model_dir).into()),
                lexicon: Some(format!(
                    "{}/lexicon-us-en.txt,{}/lexicon-zh.txt",
                    model_dir, model_dir
                ).into()),
                length_scale: 1.0,
                ..Default::default()
            },
            num_threads: 4,
            debug: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let tts = OfflineTts::create(&config).expect("Failed to create OfflineTts");

    println!("Sample rate: {}", tts.sample_rate());
    println!("Num speakers: {}", tts.num_speakers());

    // 测试文本
    let text = "你好！这是中文语音合成测试。Hello! This is English speech synthesis test. 玉米糊!";

    let gen_config = GenerationConfig {
        sid: 0,
        speed: 1.0,
        ..Default::default()
    };

    println!("\nSynthesizing: {}\n", text);

    let start = Instant::now();

    let audio = tts
        .generate_with_config(
            text,
            &gen_config,
            Some(|_samples: &[f32], progress: f32| -> bool {
                print!("\rProgress: {:.1}%", progress * 100.0);
                std::io::stdout().flush().unwrap();
                true
            }),
        )
        .expect("Generation failed");

    println!("\n\nDone!");

    let elapsed_seconds = start.elapsed().as_secs_f32();
    let duration = audio.samples().len() as f32 / audio.sample_rate() as f32;
    let rtf = elapsed_seconds / duration;

    println!("Elapsed: {:.3}s, Duration: {:.3}s, RTF: {:.3}", elapsed_seconds, duration, rtf);

    let filename = "generated_tts.wav";
    if audio.save(filename) {
        println!("Saved to: {}", filename);
    } else {
        eprintln!("Failed to save {}", filename);
    }
}
