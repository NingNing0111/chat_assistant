//! Keyword Spotter Example - 测试唤醒词检测
//!
//! 运行: cargo run --example keyword_spotter_example

use clap::Parser;
use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, Wave};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// WAV file to test (optional, will use microphone if not provided)
    #[arg(long, default_value = "")]
    wav: String,

    /// Keywords to detect (comma separated)
    #[arg(long, default_value = "")]
    keywords: String,
}

fn detect_from_wav(
    kws: &KeywordSpotter,
    wav_path: &str,
    extra_keywords: Option<&str>,
) -> anyhow::Result<()> {
    let wave = Wave::read(wav_path).ok_or_else(|| anyhow::anyhow!("Failed to read WAV"))?;

    println!("Testing: {}", wav_path);

    let stream = if let Some(extra) = extra_keywords {
        kws.create_stream_with_keywords(extra)
    } else {
        kws.create_stream()
    };

    let tail_padding = vec![0.0f32; (wave.sample_rate() / 2) as usize];
    stream.accept_waveform(wave.sample_rate(), wave.samples());
    stream.accept_waveform(wave.sample_rate(), &tail_padding);
    stream.input_finished();

    let mut detected = false;
    while kws.is_ready(&stream) {
        kws.decode(&stream);
        if let Some(result) = kws.get_result(&stream) {
            if !result.keyword.is_empty() {
                detected = true;
                println!("  -> Detected: {}", result.keyword);
                kws.reset(&stream);
            }
        }
    }

    if !detected {
        println!("  -> No keyword detected");
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let model_dir = "./models/wakeword";

    println!("Loading keyword spotter from: {}\n", model_dir);

    let mut config = KeywordSpotterConfig::default();
    config.model_config.transducer.encoder = Some(format!("{}/encoder.onnx", model_dir));
    config.model_config.transducer.decoder = Some(format!("{}/decoder.onnx", model_dir));
    config.model_config.transducer.joiner = Some(format!("{}/joiner.onnx", model_dir));
    config.model_config.tokens = Some(format!("{}/tokens.txt", model_dir));
    config.model_config.provider = Some("cpu".into());
    config.model_config.num_threads = 2;
    config.model_config.debug = false;
    config.keywords_file = Some(format!("{}/keywords.txt", model_dir));

    let kws = KeywordSpotter::create(&config)
        .ok_or_else(|| anyhow::anyhow!("Failed to create KeywordSpotter"))?;

    println!("Keyword spotter created successfully!\n");

    // 测试WAV文件
    let test_wavs = vec![
        "test_wavs/0.wav",
        "test_wavs/1.wav",
        "test_wavs/2.wav",
        "test_wavs/3.wav",
    ];

    for wav in test_wavs {
        let full_path = format!("{}/{}", model_dir, wav);
        if std::path::Path::new(&full_path).exists() {
            detect_from_wav(&kws, &full_path, None)?;
        }
    }

    // 如果有关键词参数，测试额外关键词
    if !args.keywords.is_empty() {
        println!("\nTesting with custom keywords: {}", args.keywords);
        let full_path = format!("{}/test_wavs/3.wav", model_dir);
        detect_from_wav(&kws, &full_path, Some(&args.keywords))?;
    }

    println!("\nDone!");

    Ok(())
}
