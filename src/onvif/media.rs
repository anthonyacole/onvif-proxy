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
        let fixed_response = Self::normalize_profiles(&response);

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

        // Reolink cameras typically return correct RTSP URLs, but we may need to validate
        let fixed_response = Self::fix_stream_uri_response(&response);

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

        Ok(response)
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

    fn fix_stream_uri_response(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Ensure MediaUri has proper namespace
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:Uri>") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tt="http://www.onvif.org/ver10/schema""#,
            );
        }

        fixed
    }
}
