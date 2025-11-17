use anyhow::{Context, Result};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

use crate::camera::CameraManager;
use crate::onvif::events::EventsService;
use crate::server::routes::{create_router, AppState};

pub async fn start_server(
    listen_addr: String,
    base_url: String,
    camera_manager: CameraManager,
) -> Result<()> {
    let addr: SocketAddr = listen_addr
        .parse()
        .context("Failed to parse listen address")?;

    let events_service = EventsService::new();

    let state = AppState {
        camera_manager,
        events_service,
        base_url,
    };

    let app = create_router(state)
        .layer(TraceLayer::new_for_http());

    tracing::info!("Starting ONVIF proxy server on {}", addr);
    tracing::info!("Access cameras at: /onvif/{{camera_id}}/{{service}}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Failed to bind to address")?;

    axum::serve(listener, app)
        .await
        .context("Server error")?;

    Ok(())
}
