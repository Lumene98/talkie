use backend::{app, state::Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_owned());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!(
        "Walkie-talkie backend listening on {}",
        listener.local_addr()?
    );
    axum::serve(listener, app(Config::default())).await?;
    Ok(())
}
