use std::{path::PathBuf, time::Duration};

const SAMPLES: [&str; 3] = ["01_hello.wav", "02_numbers.wav", "03_long_message.wav"];

fn sample_pcm(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../asr/samples")
        .join(name);
    let mut wav = hound::WavReader::open(&path)
        .unwrap_or_else(|error| panic!("Cannot open {}: {error}", path.display()));
    let spec = wav.spec();
    assert_eq!(spec.channels, 1, "{name}: expected mono audio");
    assert_eq!(spec.sample_rate, 16_000, "{name}: expected 16 kHz");
    assert_eq!(spec.bits_per_sample, 16, "{name}: expected 16-bit PCM");
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);

    // transcribe() takes raw PCM and creates its own WAV header. Passing the
    // entire fixture file would incorrectly treat the input header as audio.
    let pcm: Vec<u8> = wav
        .samples::<i16>()
        .map(|sample| sample.expect("valid PCM sample"))
        .flat_map(i16::to_le_bytes)
        .collect();
    assert!(!pcm.is_empty(), "{name}: empty audio");
    assert!(pcm.len() <= 480_000, "{name}: exceeds the 15-second limit");
    pcm
}

#[test]
fn samples_match_the_relay_audio_format() {
    for sample in SAMPLES {
        sample_pcm(sample);
    }
}

#[tokio::test]
#[ignore = "requires the ASR service and loaded model at http://127.0.0.1:8001"]
async fn transcribes_sample_wavs() {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(130))
        .build()
        .expect("build ASR client");

    // Sequential requests avoid competing for the local inference model.
    for sample in SAMPLES {
        let response = backend::asr::transcribe(&client, &sample_pcm(sample))
            .await
            .unwrap_or_else(|error| panic!("{sample}: transcription failed: {error}"));
        let text = response
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("{sample}: expected Ollama message.content, got {response}"));
        assert!(!text.trim().is_empty(), "{sample}: empty transcript");
        println!("{sample}: {text}");
    }
}
