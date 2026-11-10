pub async fn transcribe(
    client: &reqwest::Client,
    wav: Vec<u8>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let file = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;

    let form = reqwest::multipart::Form::new().part("file", file);

    let response = client
        .post("http://127.0.0.1:8001/transcribe")
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}
