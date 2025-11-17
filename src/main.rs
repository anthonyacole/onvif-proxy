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
    let base_url = std::env::var("BASE_URL")
        .unwrap_or_else(|_| format!("http://{}", config.proxy.listen_address));

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
