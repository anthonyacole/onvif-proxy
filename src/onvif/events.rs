use crate::camera::CameraClient;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct Subscription {
    pub _subscription_ref: String,
    pub _camera_id: String,
    pub _created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
}

pub struct EventsService {
    subscriptions: Arc<RwLock<HashMap<String, Subscription>>>,
}

impl EventsService {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_event_properties(camera: &CameraClient) -> Result<String> {
        let request_body = r#"<tev:GetEventProperties xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#;

        let response = camera
            .send_soap_request("/onvif/event_service", request_body)
            .await?;

        // Fix Reolink's custom event topics to standard ONVIF topics
        let fixed_response = Self::normalize_event_properties(&response);

        Ok(fixed_response)
    }

    pub async fn create_pull_point_subscription(
        &self,
        camera: &CameraClient,
        base_url: &str,
    ) -> Result<String> {
        let request_body = r#"<tev:CreatePullPointSubscription xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
  <tev:InitialTerminationTime>PT600S</tev:InitialTerminationTime>
</tev:CreatePullPointSubscription>"#;

        let response = camera
            .send_soap_request("/onvif/event_service", request_body)
            .await?;

        // Extract subscription reference from response and rewrite it to point to our proxy
        let subscription_ref = Uuid::new_v4().to_string();
        let subscription = Subscription {
            _subscription_ref: subscription_ref.clone(),
            _camera_id: camera.camera_id().to_string(),
            _created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(600),
        };

        self.subscriptions.write().await.insert(subscription_ref.clone(), subscription);

        // Rewrite the subscription reference URL to point to our proxy
        let proxy_subscription_url = format!("{}/onvif/{}/subscription/{}", base_url, camera.camera_id(), subscription_ref);
        let fixed_response = Self::rewrite_subscription_ref(&response, &proxy_subscription_url);

        Ok(fixed_response)
    }

    pub async fn pull_messages(
        &self,
        camera: &CameraClient,
        timeout: &str,
        message_limit: i32,
    ) -> Result<String> {
        let request_body = format!(
            r#"<tev:PullMessages xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
  <tev:Timeout>{}</tev:Timeout>
  <tev:MessageLimit>{}</tev:MessageLimit>
</tev:PullMessages>"#,
            timeout, message_limit
        );

        let response = camera
            .send_soap_request("/onvif/event_service", &request_body)
            .await?;

        // Translate Reolink's custom events to standard ONVIF events
        let fixed_response = Self::translate_event_messages(&response);

        Ok(fixed_response)
    }

    pub async fn renew_subscription(
        &self,
        camera: &CameraClient,
        subscription_ref: &str,
    ) -> Result<String> {
        let request_body = r#"<tev:Renew xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
  <tev:TerminationTime>PT600S</tev:TerminationTime>
</tev:Renew>"#;

        let response = camera
            .send_soap_request("/onvif/event_service", request_body)
            .await?;

        // Update subscription expiry time
        if let Some(sub) = self.subscriptions.write().await.get_mut(subscription_ref) {
            sub.expires_at = Utc::now() + chrono::Duration::seconds(600);
        }

        Ok(response)
    }

    pub async fn unsubscribe(
        &self,
        camera: &CameraClient,
        subscription_ref: &str,
    ) -> Result<String> {
        let request_body = r#"<tev:Unsubscribe xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#;

        let response = camera
            .send_soap_request("/onvif/event_service", request_body)
            .await?;

        // Remove subscription from our tracking
        self.subscriptions.write().await.remove(subscription_ref);

        Ok(response)
    }

    fn normalize_event_properties(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Ensure proper ONVIF event namespaces
        if !fixed.contains("xmlns:tns1=") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tns1="http://www.onvif.org/ver10/topics""#,
            );
        }

        // Map Reolink-specific topics to standard ONVIF topics
        // Reolink uses proprietary event topics for smart detection
        fixed = Self::map_reolink_topics(&fixed);

        fixed
    }

    fn map_reolink_topics(xml: &str) -> String {
        let mut result = xml.to_string();

        // Common Reolink smart detection events and their ONVIF equivalents
        let topic_mappings = vec![
            // Reolink person detection -> ONVIF CellMotionDetector
            ("RuleEngine/MyRuleDetector/PeopleDetect", "RuleEngine/CellMotionDetector/Motion"),
            ("RuleEngine/MyRuleDetector/VehicleDetect", "RuleEngine/CellMotionDetector/Motion"),
            ("RuleEngine/MyRuleDetector/DogCatDetect", "RuleEngine/CellMotionDetector/Motion"),
            // Reolink uses custom namespaces, map to tns1
            ("xmlns:reo=", "xmlns:tns1="),
            ("<reo:", "<tns1:"),
            ("</reo:", "</tns1:"),
        ];

        for (reolink_pattern, onvif_pattern) in topic_mappings {
            result = result.replace(reolink_pattern, onvif_pattern);
        }

        result
    }

    fn translate_event_messages(xml: &str) -> String {
        let mut fixed = xml.to_string();

        // Add missing namespaces for event messages
        if !fixed.contains("xmlns:tns1=") && fixed.contains("tns1:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tns1="http://www.onvif.org/ver10/topics""#,
            );
        }

        if !fixed.contains("xmlns:tt=") && fixed.contains("tt:") {
            fixed = fixed.replace(
                "<SOAP-ENV:Envelope",
                r#"<SOAP-ENV:Envelope xmlns:tt="http://www.onvif.org/ver10/schema""#,
            );
        }

        // Translate Reolink event data to ONVIF format
        fixed = Self::map_reolink_topics(&fixed);

        // Ensure event messages have proper SimpleItem structure
        fixed = Self::normalize_event_data(&fixed);

        fixed
    }

    fn normalize_event_data(xml: &str) -> String {
        // Reolink sometimes returns event data in non-standard format
        // This function normalizes it to ONVIF's expected SimpleItem structure
        let mut result = xml.to_string();

        // Add IsMotion SimpleItem if missing (required by many NVRs)
        if result.contains("Motion") && !result.contains("IsMotion") {
            // This is a simplified approach - in production, you'd parse XML properly
            result = result.replace(
                "</tev:Data>",
                r#"<tt:SimpleItem Name="IsMotion" Value="true"/></tev:Data>"#,
            );
        }

        result
    }

    fn rewrite_subscription_ref(xml: &str, proxy_url: &str) -> String {
        // Replace the camera's subscription URL with our proxy URL
        let mut result = xml.to_string();

        // Find and replace SubscriptionReference/Address
        if let Some(start) = result.find("<Address>") {
            if let Some(end) = result[start..].find("</Address>") {
                let end_pos = start + end;
                let before = &result[..start + 9]; // length of "<Address>"
                let after = &result[end_pos..];
                result = format!("{}{}{}", before, proxy_url, after);
            }
        }

        result
    }
}

impl Default for EventsService {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventsService {
    fn clone(&self) -> Self {
        Self {
            subscriptions: Arc::clone(&self.subscriptions),
        }
    }
}
