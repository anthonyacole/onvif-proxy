use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};

use crate::camera::CameraManager;
use crate::onvif::{device, media, events, soap::SoapEnvelope};
use crate::translator::ResponseTranslator;

#[derive(Clone)]
pub struct AppState {
    pub camera_manager: CameraManager,
    pub events_service: events::EventsService,
    pub base_url: String,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Device service endpoints
        .route("/onvif/:camera_id/device_service", post(handle_device_service))
        // Media service endpoints
        .route("/onvif/:camera_id/media_service", post(handle_media_service))
        // Events service endpoints
        .route("/onvif/:camera_id/event_service", post(handle_events_service))
        // Subscription endpoints
        .route("/onvif/:camera_id/subscription/:sub_id", post(handle_subscription))
        // Health check
        .route("/health", axum::routing::get(health_check))
        .with_state(state)
}

async fn handle_device_service(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    body: String,
) -> Response {
    tracing::info!("Device service request for camera: {}", camera_id);
    tracing::debug!("Request body: {}", body);

    let camera = match state.camera_manager.get_camera(&camera_id).await {
        Some(cam) => cam,
        None => {
            tracing::error!("Camera not found: {}", camera_id);
            return (StatusCode::NOT_FOUND, "Camera not found").into_response();
        }
    };

    // Parse SOAP request
    let envelope = match SoapEnvelope::parse(&body) {
        Ok(env) => env,
        Err(e) => {
            tracing::error!("Failed to parse SOAP request: {}", e);
            return (StatusCode::BAD_REQUEST, format!("Invalid SOAP: {}", e)).into_response();
        }
    };

    let action = envelope.extract_action();
    tracing::info!("Device action: {}", action);

    // Handle empty action (probe requests)
    if action.is_empty() {
        tracing::debug!("Empty action - likely a probe request");
        return (StatusCode::OK, "OK").into_response();
    }

    let response = match action.as_str() {
        "GetDeviceInformation" => {
            device::DeviceService::get_device_information(&camera, &state.base_url).await
        }
        "GetCapabilities" => {
            device::DeviceService::get_capabilities(&camera, &state.base_url).await
        }
        "GetServices" => {
            device::DeviceService::get_services(&camera, &state.base_url).await
        }
        _ => {
            tracing::warn!("Unknown device action: {}", action);
            return (StatusCode::NOT_IMPLEMENTED, format!("Action not implemented: {}", action)).into_response();
        }
    };

    match response {
        Ok(xml) => {
            // Apply translation quirks
            let quirks = camera.config().quirks.clone();
            let translated = match ResponseTranslator::translate(&xml, &camera.config().model, &quirks) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Translation failed: {}", e);
                    xml
                }
            };

            (StatusCode::OK, translated).into_response()
        }
        Err(e) => {
            tracing::error!("Device service error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn handle_media_service(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    body: String,
) -> Response {
    tracing::info!("Media service request for camera: {}", camera_id);
    tracing::debug!("Request body: {}", body);

    let camera = match state.camera_manager.get_camera(&camera_id).await {
        Some(cam) => cam,
        None => {
            return (StatusCode::NOT_FOUND, "Camera not found").into_response();
        }
    };

    let envelope = match SoapEnvelope::parse(&body) {
        Ok(env) => env,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid SOAP: {}", e)).into_response();
        }
    };

    let action = envelope.extract_action();
    tracing::info!("Media action: {}", action);

    let response = match action.as_str() {
        "GetProfiles" => {
            media::MediaService::get_profiles(&camera).await
        }
        "GetStreamUri" => {
            // Extract profile token and protocol from request
            let profile_token = extract_value(&body, "ProfileToken").unwrap_or("Profile_1".to_string());
            let protocol = extract_value(&body, "Protocol").unwrap_or("RTSP".to_string());
            media::MediaService::get_stream_uri(&camera, &profile_token, &protocol).await
        }
        "GetSnapshotUri" => {
            let profile_token = extract_value(&body, "ProfileToken").unwrap_or("Profile_1".to_string());
            media::MediaService::get_snapshot_uri(&camera, &profile_token).await
        }
        _ => {
            tracing::warn!("Unknown media action: {}", action);
            return (StatusCode::NOT_IMPLEMENTED, format!("Action not implemented: {}", action)).into_response();
        }
    };

    match response {
        Ok(xml) => {
            let quirks = camera.config().quirks.clone();
            let translated = match ResponseTranslator::translate(&xml, &camera.config().model, &quirks) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Translation failed: {}", e);
                    xml
                }
            };

            (StatusCode::OK, translated).into_response()
        }
        Err(e) => {
            tracing::error!("Media service error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn handle_events_service(
    State(state): State<AppState>,
    Path(camera_id): Path<String>,
    body: String,
) -> Response {
    tracing::info!("Events service request for camera: {}", camera_id);
    tracing::debug!("Request body: {}", body);

    let camera = match state.camera_manager.get_camera(&camera_id).await {
        Some(cam) => cam,
        None => {
            return (StatusCode::NOT_FOUND, "Camera not found").into_response();
        }
    };

    let envelope = match SoapEnvelope::parse(&body) {
        Ok(env) => env,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid SOAP: {}", e)).into_response();
        }
    };

    let action = envelope.extract_action();
    tracing::info!("Events action: {}", action);

    let response = match action.as_str() {
        "GetEventProperties" => {
            events::EventsService::get_event_properties(&camera).await
        }
        "CreatePullPointSubscription" => {
            state.events_service.create_pull_point_subscription(&camera, &state.base_url).await
        }
        "PullMessages" => {
            let timeout = extract_value(&body, "Timeout").unwrap_or("PT1S".to_string());
            let message_limit = extract_value(&body, "MessageLimit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10);
            state.events_service.pull_messages(&camera, &timeout, message_limit).await
        }
        "Renew" => {
            let sub_ref = ""; // Extract from request
            state.events_service.renew_subscription(&camera, sub_ref).await
        }
        "Unsubscribe" => {
            let sub_ref = ""; // Extract from request
            state.events_service.unsubscribe(&camera, sub_ref).await
        }
        _ => {
            tracing::warn!("Unknown events action: {}", action);
            return (StatusCode::NOT_IMPLEMENTED, format!("Action not implemented: {}", action)).into_response();
        }
    };

    match response {
        Ok(xml) => {
            let quirks = camera.config().quirks.clone();
            let translated = match ResponseTranslator::translate(&xml, &camera.config().model, &quirks) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Translation failed: {}", e);
                    xml
                }
            };

            (StatusCode::OK, translated).into_response()
        }
        Err(e) => {
            tracing::error!("Events service error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn handle_subscription(
    State(state): State<AppState>,
    Path((camera_id, sub_id)): Path<(String, String)>,
    body: String,
) -> Response {
    tracing::info!("Subscription request for camera: {}, subscription: {}", camera_id, sub_id);

    let camera = match state.camera_manager.get_camera(&camera_id).await {
        Some(cam) => cam,
        None => {
            return (StatusCode::NOT_FOUND, "Camera not found").into_response();
        }
    };

    let envelope = match SoapEnvelope::parse(&body) {
        Ok(env) => env,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid SOAP: {}", e)).into_response();
        }
    };

    let action = envelope.extract_action();

    let response = match action.as_str() {
        "PullMessages" => {
            let timeout = extract_value(&body, "Timeout").unwrap_or("PT1S".to_string());
            let message_limit = extract_value(&body, "MessageLimit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10);
            state.events_service.pull_messages(&camera, &timeout, message_limit).await
        }
        "Renew" => {
            state.events_service.renew_subscription(&camera, &sub_id).await
        }
        "Unsubscribe" => {
            state.events_service.unsubscribe(&camera, &sub_id).await
        }
        _ => {
            return (StatusCode::NOT_IMPLEMENTED, format!("Action not implemented: {}", action)).into_response();
        }
    };

    match response {
        Ok(xml) => (StatusCode::OK, xml).into_response(),
        Err(e) => {
            tracing::error!("Subscription error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// Helper function to extract values from XML (simplified)
fn extract_value(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    if let Some(start) = xml.find(&start_tag) {
        let content_start = start + start_tag.len();
        if let Some(end) = xml[content_start..].find(&end_tag) {
            return Some(xml[content_start..content_start + end].trim().to_string());
        }
    }

    // Try with namespace prefix
    for prefix in &["trt:", "tev:", "tds:", "tt:"] {
        let start_tag = format!("<{}{}>", prefix, tag);
        let end_tag = format!("</{}{}>", prefix, tag);

        if let Some(start) = xml.find(&start_tag) {
            let content_start = start + start_tag.len();
            if let Some(end) = xml[content_start..].find(&end_tag) {
                return Some(xml[content_start..content_start + end].trim().to_string());
            }
        }
    }

    None
}
