use chainless_lb_backend::config;
use chainless_lb_backend::observability::init_logging;
use chainless_lb_backend::server::build_server;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"));
    let config = config::load_config(Some(&config_path))?;
    init_logging(&config.logging);
    tracing::info!("Starting Chainless LB Backend with config: {:?}", config);

    let server = build_server(&config)?;
    tracing::info!("Server starting…");
    server.run_forever()
}
