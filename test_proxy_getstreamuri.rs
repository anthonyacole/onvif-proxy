// Test what the proxy returns for GetStreamUri after translation
use onvif_proxy::camera::{CameraClient, CameraConfig};
use onvif_proxy::translator::ResponseTranslator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = CameraConfig {
        id: "test".to_string(),
        name: "Test Camera".to_string(),
        address: "192.168.30.11:8000".to_string(),
        username: "stream".to_string(),
        password: "111111".to_string(),
        model: "reolink".to_string(),
        enable_smart_detection: false,
        quirks: vec![
            "fix_device_info_namespace".to_string(),
            "normalize_media_profiles".to_string(),
            "translate_smart_events".to_string(),
        ],
    };

    let camera = CameraClient::new(config.clone());

    println!("\n=== Testing GetStreamUri through proxy translation ===");
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
        Ok(raw_response) => {
            println!("\n--- RAW RESPONSE FROM CAMERA ---");
            println!("{}", raw_response);

            // Apply translation with quirks (simulating what the proxy does)
            let quirks = config.quirks.clone();
            let translated = match ResponseTranslator::translate(&raw_response, &config.model, &quirks) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Translation failed: {}", e);
                    raw_response.clone()
                }
            };

            println!("\n--- TRANSLATED RESPONSE (after quirks) ---");
            println!("{}", translated);

            // Check if they're different
            if raw_response == translated {
                println!("\n✓ Translation did not modify the response (GOOD)");
            } else {
                println!("\n✗ Translation MODIFIED the response:");
                println!("  Length changed: {} -> {} bytes", raw_response.len(), translated.len());
            }

            // Extract and display the URI
            if let Some(start) = translated.find("<tt:Uri>") {
                if let Some(end) = translated[start..].find("</tt:Uri>") {
                    let uri = &translated[start + 8..start + end];
                    println!("\n📹 Extracted URI: {}", uri);
                }
            } else {
                println!("\n⚠ WARNING: Could not find <tt:Uri> in response!");
            }
        }
        Err(e) => {
            eprintln!("GetStreamUri Error: {}", e);
        }
    }

    Ok(())
}
