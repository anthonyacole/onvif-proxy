use anyhow::{Context, Result};
use reqwest::Client;
use crate::onvif::auth::WsSecurityAuth;
use crate::camera::config::CameraConfig;

#[derive(Clone)]
pub struct CameraClient {
    config: CameraConfig,
    http_client: Client,
    auth: WsSecurityAuth,
}

impl CameraClient {
    pub fn new(config: CameraConfig) -> Self {
        let auth = WsSecurityAuth::new(config.username.clone(), config.password.clone());
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            auth,
        }
    }

    pub async fn send_soap_request(&self, service_path: &str, soap_body: &str) -> Result<String> {
        let url = format!("{}{}", self.config.base_url(), service_path);

        // Create SOAP envelope with WS-Security header
        let security_header = self.auth.generate_header();
        let authenticated_soap = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema" xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
<SOAP-ENV:Header>
{}
</SOAP-ENV:Header>
<SOAP-ENV:Body>
{}
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>"#,
            security_header, soap_body
        );

        tracing::debug!("Sending SOAP request to {}: {}", url, authenticated_soap);

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(authenticated_soap)
            .send()
            .await
            .context("Failed to send SOAP request to camera")?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .context("Failed to read response from camera")?;

        if !status.is_success() {
            tracing::warn!("Camera returned error status {}: {}", status, response_text);
        }

        tracing::debug!("Received SOAP response from camera: {}", response_text);

        Ok(response_text)
    }

    pub fn camera_id(&self) -> &str {
        &self.config.id
    }

    pub fn config(&self) -> &CameraConfig {
        &self.config
    }
}
