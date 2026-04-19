use chainless_lb_backend::config;
use chainless_lb_backend::server::build_server;
use goose::prelude::*;
use std::time::Duration;
use tokio::time::sleep;

async fn load_test_public_endpoint(user: &mut GooseUser) -> TransactionResult {
    // Tests rate-limiting / bypass parsing overhead
    let _response = user.get("/public/health").await?;
    Ok(())
}

async fn load_test_authenticated_endpoint(user: &mut GooseUser) -> TransactionResult {
    // Needs a valid dummy token or tests 401 rejection speed (still an RPS test)
    let _response = user.get("/api/data").await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Boot server in background using our benchmark base config
    let config_path = format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"));
    let app_config = config::load_config(Some(&config_path))?;
    
    // Mode switcher based on ENV or args could go here. Let's just run it!
    println!("Starting local Chainless-LB for RPS benchmarking on port 3006...");
    let server = build_server(&app_config)?;
    tokio::spawn(async move {
        server.run_forever();
    });

    sleep(Duration::from_secs(1)).await; // Allow server to boot

    println!("Running Goose Benchmark Suite. You can append CLI arguments like: --host http://127.0.0.1:3006 -u 100 -r 10 -t 10s");

    GooseAttack::initialize()?
        .register_scenario(scenario!("PublicAccess")
            .set_weight(10)?
            .register_transaction(transaction!(load_test_public_endpoint))
        )
        .register_scenario(scenario!("AuthAccess")
            .set_weight(5)?
            .register_transaction(transaction!(load_test_authenticated_endpoint))
        )
        .set_default(GooseDefault::Host, "http://127.0.0.1:3006")?
        .execute()
        .await?;

    Ok(())
}
