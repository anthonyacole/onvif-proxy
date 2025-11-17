// Simple test program to query GetStreamUri directly from the camera
// Run with: cargo run --bin test_getstreamuri

use onvif_proxy::camera::{CameraClient, CameraConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Create camera config
    let config = CameraConfig {
        id: "test".to_string(),
        name: "Test Camera".to_string(),
        address: "192.168.30.11:8000".to_string(),
        username: "stream".to_string(),
        password: "111111".to_string(),
        model: "reolink".to_string(),
        enable_smart_detection: false,
        quirks: vec![],
    };

    // Create camera client
    let camera = CameraClient::new(config);

    println!("\n=== Testing GetProfiles ===");
    let profiles_request = r#"<trt:GetProfiles xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>"#;
    match camera.send_soap_request("/onvif/media_service", profiles_request).await {
        Ok(response) => {
            println!("GetProfiles Response:");
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("GetProfiles Error: {}", e);
        }
    }

    println!("\n=== Testing GetStreamUri (Profile 000) ===");
    let stream_uri_request = r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:StreamSetup>
    <tt:Stream xmlns:tt="http://www.onvif.org/ver10/schema">RTP-Unicast</tt:Stream>
    <tt:Transport xmlns:tt="http://www.onvif.org/ver10/schema">
      <tt:Protocol>RTSP</tt:Protocol>
    </tt:Transport>
  </trt:StreamSetup>
  <trt:ProfileToken>000</trt:ProfileToken>
</trt:GetStreamUri>"#;

    match camera.send_soap_request("/onvif/media_service", stream_uri_request).await {
        Ok(response) => {
            println!("GetStreamUri Response:");
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("GetStreamUri Error: {}", e);
        }
    }

    println!("\n=== Testing GetStreamUri (Profile 001) ===");
    let stream_uri_request_001 = r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:StreamSetup>
    <tt:Stream xmlns:tt="http://www.onvif.org/ver10/schema">RTP-Unicast</tt:Stream>
    <tt:Transport xmlns:tt="http://www.onvif.org/ver10/schema">
      <tt:Protocol>RTSP</tt:Protocol>
    </tt:Transport>
  </trt:StreamSetup>
  <trt:ProfileToken>001</trt:ProfileToken>
</trt:GetStreamUri>"#;

    match camera.send_soap_request("/onvif/media_service", stream_uri_request_001).await {
        Ok(response) => {
            println!("GetStreamUri Response:");
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("GetStreamUri Error: {}", e);
        }
    }

    Ok(())
}
