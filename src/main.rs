mod camera;
mod config;
mod onvif;
mod server;
mod translator;

use anyhow::{Context, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onvif_proxy=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting ONVIF Proxy for Reolink Cameras");

    // Load configuration
    let config_path = std::env::var("CONFIG_PATH")
        .unwrap_or_else(|_| "config/cameras.yaml".to_string());

    let config = config::AppConfig::load_from_file(&config_path)
        .context("Failed to load configuration")?;

    tracing::info!("Loaded configuration with {} cameras", config.cameras.len());

    // Initialize camera manager
    let camera_manager = camera::CameraManager::new();

    // Add all cameras from configuration
    for camera_config in config.cameras {
        tracing::info!("Adding camera: {} ({})", camera_config.name, camera_config.id);
        camera_manager.add_camera(camera_config).await;
    }

    // Determine base URL for the proxy
    // Priority: config file > environment variable > auto-detect
    let base_url = config
        .proxy
        .base_url
        .clone()
        .filter(|s| !s.trim().is_empty()) // Treat empty strings as None
        .or_else(|| std::env::var("BASE_URL").ok())
        .unwrap_or_else(|| {
            // Auto-detect: extract port from listen_address
            let port = config
                .proxy
                .listen_address
                .split(':')
                .nth(1)
                .unwrap_or("8000");

            // Try to get local IP, fallback to localhost
            let ip = local_ip_address::local_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string());

            let url = format!("http://{}:{}", ip, port);
            tracing::info!("Auto-detected base URL: {}", url);
            url
        });

    tracing::info!("Proxy base URL: {}", base_url);

    // Start the server
    server::start_server(
        config.proxy.listen_address.clone(),
        base_url,
        camera_manager,
    )
    .await?;

    Ok(())
}
