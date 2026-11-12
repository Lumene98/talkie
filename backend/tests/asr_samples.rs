use axum::body::Bytes;
use std::{path::PathBuf, time::Duration};

const SAMPLES: [&str; 3] = ["01_hello.wav", "02_numbers.wav", "03_long_message.wav"];

#[tokio::test]
#[ignore = "requires the ASR service and loaded model at http://127.0.0.1:8001"]
async fn transcribes_sample_wavs() {
    let url =
        std::env::var("ASR_URL").unwrap_or_else(|_| backend::state::Config::default().asr_url);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(130))
        .build()
        .expect("build ASR client");

    // Sequential requests avoid competing for the local inference model.
    for sample in SAMPLES {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../asr/samples")
            .join(sample);

        let wav_file = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("Cannot open {}: {error}", path.display()));

        let wav = hound::WavReader::open(&path)
            .unwrap_or_else(|error| panic!("Cannot open {}: {error}", path.display()));

        let spec = wav.spec();
        assert_eq!(spec.channels, 1, "{sample}: expected mono audio");
        assert_eq!(spec.sample_rate, 16_000, "{sample}: expected 16 kHz");
        assert_eq!(spec.bits_per_sample, 16, "{sample}: expected 16-bit PCM");
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);

        let text = backend::asr::transcribe(&client, &url, Bytes::from(wav_file))
            .await
            .unwrap_or_else(|error| panic!("{sample}: transcription failed: {error}"));

        assert!(!text.trim().is_empty(), "{sample}: empty transcript");

        println!("{sample}: {text}");
    }
}
