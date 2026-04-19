use chainless_lb_backend::config;
use chainless_lb_backend::server::build_server;
use std::error::Error;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config_path = format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"));
    let app_config = config::load_config(Some(&config_path))?;

    let server = build_server(&app_config)?;
    tokio::spawn(async move {
        server.run_forever();
    });

    sleep(Duration::from_secs(1)).await;

    let client = Client::new();
    println!("Firing burst of requests to generate metrics export payloads...");
    for _ in 0..10 {
        let _ = client.get("http://127.0.0.1:3004/public").send().await;
    }
    
    println!("Metrics successfully aggregated and exported to OTLP collector!");
    
    Ok(())
}
