use crate::camera::CameraClient;
use anyhow::Result;

pub struct MediaService;

impl MediaService {
    pub async fn get_profiles(camera: &CameraClient) -> Result<String> {
        let request_body = r#"<trt:GetProfiles xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>"#;

        let response = camera
            .send_soap_request("/onvif/media_service", request_body)
            .await?;

        // Fix namespace issues and normalize profile structure
        let mut fixed_response = Self::normalize_profiles(&response);

        // Fix any localhost URLs in the profile URIs
        fixed_response = Self::fix_stream_uri_response(&fixed_response, camera);

        Ok(fixed_response)
    }

    pub async fn get_stream_uri(camera: &CameraClient, profile_token: &str, protocol: &str) -> Result<String> {
        let request_body = format!(
            r#"<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:StreamSetup>
    <tt:Stream xmlns:tt="http://www.onvif.org/ver10/schema">RTP-Unicast</tt:Stream>
    <tt:Transport xmlns:tt="http://www.onvif.org/ver10/schema">
      <tt:Protocol>{}</tt:Protocol>
    </tt:Transport>
  </trt:StreamSetup>
  <trt:ProfileToken>{}</trt:ProfileToken>
</trt:GetStreamUri>"#,
            protocol, profile_token
        );

        let response = camera
            .send_soap_request("/onvif/media_service", &request_body)
            .await?;

        // Fix RTSP URLs - Reolink cameras return 127.0.0.1 instead of actual IP
        let fixed_response = Self::fix_stream_uri_response(&response, camera);

        Ok(fixed_response)
    }

    pub async fn get_snapshot_uri(camera: &CameraClient, profile_token: &str) -> Result<String> {
        let request_body = format!(
            r#"<trt:GetSnapshotUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
  <trt:ProfileToken>{}</trt:ProfileToken>
</trt:GetSnapshotUri>"#,
            profile_token
        );

        let response = camera
            .send_soap_request("/onvif/media_service", &request_body)
            .await?;

        // Fix localhost URLs in snapshot URI
        let fixed_response = Self::fix_stream_uri_response(&response, camera);

        Ok(fixed_response)
    }

    fn normalize_profiles(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Ensure proper namespace declarations
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tt="http://www.onvif.org/ver10/schema""#,
            );
        }

        if !fixed.contains("xmlns:trt=") && fixed.contains("<trt:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:trt="http://www.onvif.org/ver10/media/wsdl""#,
            );
        }

        // Reolink sometimes returns profiles without required fields
        // Add defaults if missing (this is a simplified approach)
        fixed
    }

    fn fix_stream_uri_response(xml: &str, camera: &CameraClient) -> String {
        let mut fixed = xml.to_string();

        // Ensure MediaUri has proper namespace
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:Uri>") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tt="http://www.onvif.org/ver10/schema""#,
            );
        }

        // Fix RTSP URLs - Reolink cameras return 127.0.0.1 or localhost instead of actual IP
        // Extract the camera's actual IP address from the config
        let camera_address = &camera.config().address;
        let camera_ip = if let Some(host) = camera_address.split(':').next() {
            host
        } else {
            camera_address.as_str()
        };

        // Replace localhost references in RTSP URLs with actual camera IP
        fixed = fixed.replace("rtsp://127.0.0.1:", &format!("rtsp://{}:", camera_ip));
        fixed = fixed.replace("rtsp://localhost:", &format!("rtsp://{}:", camera_ip));
        fixed = fixed.replace("rtsp://0.0.0.0:", &format!("rtsp://{}:", camera_ip));

        // Also fix HTTP URLs for snapshot URIs if they have localhost
        fixed = fixed.replace("http://127.0.0.1:", &format!("http://{}:", camera_ip));
        fixed = fixed.replace("http://localhost:", &format!("http://{}:", camera_ip));
        fixed = fixed.replace("http://0.0.0.0:", &format!("http://{}:", camera_ip));

        fixed
    }
}
