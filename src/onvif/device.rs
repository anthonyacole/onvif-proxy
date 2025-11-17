use crate::camera::CameraClient;
use anyhow::Result;

pub struct DeviceService;

impl DeviceService {
    pub async fn get_device_information(camera: &CameraClient, _base_url: &str) -> Result<String> {
        let request_body = r#"<tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>"#;

        let response = camera
            .send_soap_request("/onvif/device_service", request_body)
            .await?;

        // Fix namespace issues in Reolink response
        let fixed_response = Self::fix_device_info_namespaces(&response);

        Ok(fixed_response)
    }

    pub async fn get_capabilities(camera: &CameraClient, base_url: &str) -> Result<String> {
        let request_body = r#"<tds:GetCapabilities xmlns:tds="http://www.onvif.org/ver10/device/wsdl"><tds:Category>All</tds:Category></tds:GetCapabilities>"#;

        let response = camera
            .send_soap_request("/onvif/device_service", request_body)
            .await?;

        // Rewrite XAddr URLs to point to our proxy instead of the camera
        let fixed_response = Self::rewrite_capability_urls(&response, &camera.config().id, base_url);

        Ok(fixed_response)
    }

    pub async fn get_services(camera: &CameraClient, base_url: &str) -> Result<String> {
        let request_body = r#"<tds:GetServices xmlns:tds="http://www.onvif.org/ver10/device/wsdl"><tds:IncludeCapability>true</tds:IncludeCapability></tds:GetServices>"#;

        let response = camera
            .send_soap_request("/onvif/device_service", request_body)
            .await?;

        // Rewrite service URLs to point to our proxy
        let fixed_response = Self::rewrite_service_urls(&response, &camera.config().id, base_url);

        Ok(fixed_response)
    }

    fn fix_device_info_namespaces(xml: &str) -> String {
        // Reolink often returns responses with missing or incorrect namespace declarations
        // Ensure the response has proper ONVIF namespaces
        let mut fixed = xml.to_string();

        // Add missing tt namespace if not present
        if !fixed.contains("xmlns:tt=") && fixed.contains("<tt:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tt="http://www.onvif.org/ver10/schema""#,
            );
        }

        // Add missing tds namespace if not present
        if !fixed.contains("xmlns:tds=") && fixed.contains("<tds:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tds="http://www.onvif.org/ver10/device/wsdl""#,
            );
        }

        fixed
    }

    fn rewrite_capability_urls(xml: &str, camera_id: &str, base_url: &str) -> String {
        // Replace camera's internal URLs with our proxy URLs
        // This ensures iSpy talks to us instead of trying to reach the camera directly
        let mut result = xml.to_string();

        // Service path mappings
        let services = vec![
            "device_service",
            "media_service",
            "event_service",
            "ptz_service",
            "imaging_service",
            "analytics_service",
            "deviceIO_service",
            "Media2",
        ];

        for service in services {
            // Match full URL patterns like: http://192.168.30.11:8000/onvif/device_service
            // We need to replace the entire URL, not just the path, to avoid concatenation
            // Look for pattern: http://[anything]/onvif/[service]
            let path_pattern = format!("/onvif/{}", service);

            // Find all occurrences and replace the full URL
            let mut search_start = 0;
            while let Some(pos) = result[search_start..].find(&path_pattern) {
                let abs_pos = search_start + pos;

                // Find the start of the URL by searching backwards for "http://" or "https://"
                let before_path = &result[..abs_pos];
                if let Some(url_start) = before_path.rfind("http://").or_else(|| before_path.rfind("https://")) {
                    // Extract and replace the full URL
                    let old_url = &result[url_start..abs_pos + path_pattern.len()];
                    let new_url = format!("{}/onvif/{}/{}", base_url, camera_id, service);
                    result = result.replace(old_url, &new_url);
                    search_start = url_start + new_url.len();
                } else {
                    search_start = abs_pos + path_pattern.len();
                }
            }
        }

        result
    }

    fn rewrite_service_urls(xml: &str, camera_id: &str, base_url: &str) -> String {
        // Similar to capability URLs, rewrite service URLs
        Self::rewrite_capability_urls(xml, camera_id, base_url)
    }
}
