use chainless_lb_backend::config;
use chainless_lb_backend::server::build_server;
use std::error::Error;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config_path = format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"));
    let app_config = config::load_config(Some(&config_path))?;

    // Boot LB in background
    let server = build_server(&app_config)?;
    tokio::spawn(async move {
        server.run_forever();
    });

    sleep(Duration::from_secs(1)).await; // wait for server

    // Create JWT token manually
    let claims = Claims { sub: "test-user".into(), exp: 9999999999 };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"super_secret_pqc_key_123456789012")
    )?;

    let client = Client::new();

    // 1. Missing Token
    let resp = client.get("http://127.0.0.1:3001/secure-api").send().await?;
    println!("No Token Status: {}", resp.status());
    assert_eq!(resp.status().as_u16(), 401);

    // 2. Valid Token
    let resp = client.get("http://127.0.0.1:3001/secure-api")
        .header("Authorization", format!("Bearer {}", token))
        .send().await?;
    println!("Valid Token Status: {}", resp.status());
    // Upstream doesn't exist, will likely yield 502/503 from Pingora, but Auth layer passed (not 401!)
    assert_ne!(resp.status().as_u16(), 401);

    println!("JWT Example completed successfully!");
    Ok(())
}
