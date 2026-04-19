use chainless_lb_backend::config;
use chainless_lb_backend::server::build_server;
use std::error::Error;
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

    sleep(Duration::from_millis(500)).await;

    println!("gRPC forwarding proxy active on 3003.");
    println!("Use `tonic` client to bind to http://127.0.0.1:3003. Proxy will tunnel ALPN=h2 to port 50051 automatically.");
    
    // Kept alive for demonstration
    sleep(Duration::from_secs(60)).await;
    
    Ok(())
}
