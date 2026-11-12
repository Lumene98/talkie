use axum::body::Bytes;

pub async fn transcribe(
    client: &reqwest::Client,
    url: &str,
    wav: Bytes,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let file = reqwest::multipart::Part::stream(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new().part("file", file);

    let response = client
        .post(url)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    let response: serde_json::Value = response.json().await?;
    let content = response
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .ok_or("ASR response is missing message.content")?;

    let text = content
        .split_once("<asr_text>")
        .map(|(_, transcript)| transcript)
        .unwrap_or(content)
        .trim();

    if text.is_empty() {
        return Err("ASR returned an empty transcript".into());
    }

    Ok(text.to_owned())
}
