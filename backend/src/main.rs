use backend::{app, state::Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let mut config = Config::default();
    if let Ok(url) = std::env::var("ASR_URL") {
        let parsed = reqwest::Url::parse(&url)?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("ASR_URL must be an absolute HTTP(S) URL".into());
        }
        config.asr_url = url;
    }
    if let Ok(seconds) = std::env::var("ASR_TIMEOUT_SECS") {
        let seconds: u64 = seconds.parse()?;
        if seconds == 0 {
            return Err("ASR_TIMEOUT_SECS must be greater than zero".into());
        }
        config.asr_timeout = std::time::Duration::from_secs(seconds);
    }
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!(
        "Walkie-talkie backend listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app(config)).await?;
    Ok(())
}
