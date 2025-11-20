use crate::camera::CameraClient;
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct CachedEvent {
    pub event_xml: String,
    pub received_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub _subscription_ref: String,
    pub camera_id: String,
    pub camera_subscription_url: String,  // Original subscription URL from camera
    pub _created_at: chrono::DateTime<Utc>,
    pub expires_at: chrono::DateTime<Utc>,
    pub event_cache: Arc<RwLock<VecDeque<CachedEvent>>>,  // Cache of events from camera
    pub last_poll: Arc<RwLock<chrono::DateTime<Utc>>>,  // Last time we polled the camera
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

        // Extract camera's subscription URL from response
        let camera_subscription_url = Self::extract_subscription_url(&response);

        // Extract subscription reference from response and rewrite it to point to our proxy
        let subscription_ref = Uuid::new_v4().to_string();
        let subscription = Subscription {
            _subscription_ref: subscription_ref.clone(),
            camera_id: camera.camera_id().to_string(),
            camera_subscription_url: camera_subscription_url.clone(),
            _created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(600),
            event_cache: Arc::new(RwLock::new(VecDeque::new())),
            last_poll: Arc::new(RwLock::new(Utc::now())),
        };

        self.subscriptions.write().await.insert(subscription_ref.clone(), subscription.clone());

        // Start background polling task for this subscription
        let camera_clone = camera.clone();
        let subscription_clone = subscription.clone();
        tokio::spawn(async move {
            Self::poll_camera_events_background(camera_clone, subscription_clone).await;
        });

        // Rewrite the subscription reference URL to point to our proxy
        let proxy_subscription_url = format!("{}/onvif/{}/subscription/{}", base_url, camera.camera_id(), subscription_ref);
        let fixed_response = Self::rewrite_subscription_ref(&response, &proxy_subscription_url);

        Ok(fixed_response)
    }

    pub async fn get_subscription(&self, subscription_ref: &str) -> Option<Subscription> {
        self.subscriptions.read().await.get(subscription_ref).cloned()
    }

    pub async fn pull_messages(
        &self,
        subscription_ref: &str,
        timeout: &str,
        message_limit: i32,
    ) -> Result<String> {
        let subscription = self.get_subscription(subscription_ref).await
            .ok_or_else(|| anyhow::anyhow!("Subscription not found"))?;

        // Parse timeout to determine how long to wait for events
        let timeout_secs = Self::parse_iso_duration(timeout).unwrap_or(1);
        let deadline = Utc::now() + chrono::Duration::seconds(timeout_secs);

        // Check cache for events, waiting up to timeout if needed
        let mut events = Vec::new();
        let start = std::time::Instant::now();

        while events.len() < message_limit as usize && Utc::now() < deadline {
            {
                let mut cache = subscription.event_cache.write().await;
                while events.len() < message_limit as usize {
                    if let Some(event) = cache.pop_front() {
                        events.push(event);
                    } else {
                        break;
                    }
                }
            }

            if events.is_empty() && start.elapsed().as_secs() < timeout_secs as u64 {
                // Wait a bit for events to arrive
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            } else {
                break;
            }
        }

        // Build PullMessages response
        let response = Self::build_pull_messages_response(&events);

        Ok(response)
    }

    async fn poll_camera_events_background(camera: CameraClient, subscription: Subscription) {
        tracing::info!("Starting background event polling for subscription on camera {} (querying motion alarm state)", subscription.camera_id);

        // Track previous motion state
        let mut last_motion_state: Option<bool> = None;

        loop {
            // Poll camera every 500ms for responsive motion detection
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Query camera for current motion alarm state
            // Reolink cameras expose motion state via GetEventProperties with current state
            match Self::query_motion_state(&camera).await {
                Ok(motion_detected) => {
                    if last_motion_state != Some(motion_detected) {
                        tracing::info!("Motion state changed on camera {}: {}", subscription.camera_id, motion_detected);

                        // Generate ONVIF motion event for state change
                        let event_xml = Self::generate_motion_event(&subscription.camera_id, motion_detected);
                        let mut cache = subscription.event_cache.write().await;
                        cache.push_back(CachedEvent {
                            event_xml,
                            received_at: Utc::now(),
                        });

                        // Limit cache size
                        while cache.len() > 100 {
                            cache.pop_front();
                        }

                        last_motion_state = Some(motion_detected);
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to query motion state: {}", e);
                }
            }

            *subscription.last_poll.write().await = Utc::now();
        }
    }

    async fn query_motion_state(camera: &CameraClient) -> Result<bool> {
        // Reolink cameras have broken ONVIF PullPoint but support proprietary CGI API
        // Query motion detection state via Reolink's CGI interface (HTTPS, GET method)

        // Extract IP address from camera config (strip port if present)
        // CGI API uses HTTPS with credentials in URL query params
        let camera_address = &camera.config().address;
        let camera_ip = if let Some(host) = camera_address.split(':').next() {
            host
        } else {
            camera_address.as_str()
        };

        let username = &camera.config().username;
        let password = &camera.config().password;
        let cgi_url = format!(
            "https://{}/cgi-bin/api.cgi?cmd=GetMdState&channel=0&user={}&password={}",
            camera_ip, username, password
        );

        // Use HTTP client with SSL verification disabled (cameras use self-signed certs)
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client
            .get(&cgi_url)
            .send()
            .await?;

        let response_text = response.text().await?;

        // Parse JSON response to check motion state
        // Response format: [{"cmd":"GetMdState","code":0,"value":{"state":1}}]
        // state: 0 = no motion, 1 = motion detected
        let has_motion = response_text.contains(r#""state":1"#) ||
                        response_text.contains(r#""state": 1"#);

        Ok(has_motion)
    }

    fn generate_motion_event(camera_id: &str, motion_active: bool) -> String {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        format!(
            r#"<wsnt:NotificationMessage>
  <wsnt:Topic Dialect="http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet">tns1:RuleEngine/CellMotionDetector/Motion</wsnt:Topic>
  <wsnt:Message>
    <tt:Message UtcTime="{}">
      <tt:Source>
        <tt:SimpleItem Name="VideoSourceConfigurationToken" Value="{}"/>
        <tt:SimpleItem Name="VideoAnalyticsConfigurationToken" Value="{}"/>
        <tt:SimpleItem Name="Rule" Value="MotionDetectorRule"/>
      </tt:Source>
      <tt:Data>
        <tt:SimpleItem Name="IsMotion" Value="{}"/>
      </tt:Data>
    </tt:Message>
  </wsnt:Message>
</wsnt:NotificationMessage>"#,
            now, camera_id, camera_id, motion_active
        )
    }

    pub async fn renew_subscription(
        &self,
        camera: &CameraClient,
        camera_subscription_url: &str,
        subscription_ref: &str,
    ) -> Result<String> {
        let request_body = r#"<tev:Renew xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
  <tev:TerminationTime>PT600S</tev:TerminationTime>
</tev:Renew>"#;

        // Parse the subscription URL to extract the path
        let subscription_path = if let Some(idx) = camera_subscription_url.find("/onvif/") {
            &camera_subscription_url[idx..]
        } else {
            "/onvif/event_service"
        };

        // Reolink needs WS-Security even on subscription endpoints
        let response = camera
            .send_soap_request(subscription_path, request_body)
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
        camera_subscription_url: &str,
        subscription_ref: &str,
    ) -> Result<String> {
        let request_body = r#"<tev:Unsubscribe xmlns:tev="http://www.onvif.org/ver10/events/wsdl"/>"#;

        // Parse the subscription URL to extract the path
        let subscription_path = if let Some(idx) = camera_subscription_url.find("/onvif/") {
            &camera_subscription_url[idx..]
        } else {
            "/onvif/event_service"
        };

        let response = camera
            .send_soap_request(subscription_path, request_body)
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

    fn parse_iso_duration(duration: &str) -> Option<i64> {
        // Parse ISO 8601 duration like PT5S, PT1M, etc.
        if !duration.starts_with("PT") {
            return None;
        }
        let duration = &duration[2..];
        if duration.ends_with('S') {
            duration[..duration.len()-1].parse().ok()
        } else if duration.ends_with('M') {
            duration[..duration.len()-1].parse::<i64>().ok().map(|m| m * 60)
        } else {
            None
        }
    }

    fn extract_events_from_response(xml: &str) -> Option<Vec<String>> {
        // Extract individual NotificationMessage elements from PullMessagesResponse
        let mut events = Vec::new();
        let mut search_start = 0;

        loop {
            if let Some(msg_start) = xml[search_start..].find("<wsnt:NotificationMessage>") {
                let abs_start = search_start + msg_start;
                if let Some(msg_end) = xml[abs_start..].find("</wsnt:NotificationMessage>") {
                    let abs_end = abs_start + msg_end + "</wsnt:NotificationMessage>".len();
                    events.push(xml[abs_start..abs_end].to_string());
                    search_start = abs_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if events.is_empty() {
            None
        } else {
            Some(events)
        }
    }

    fn build_pull_messages_response(events: &[CachedEvent]) -> String {
        let mut messages = String::new();
        for event in events {
            messages.push_str(&event.event_xml);
            messages.push('\n');
        }

        let current_time = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let termination_time = (chrono::Utc::now() + chrono::Duration::seconds(600)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<SOAP-ENV:Envelope xmlns:SOAP-ENV="http://www.w3.org/2003/05/soap-envelope" xmlns:tev="http://www.onvif.org/ver10/events/wsdl" xmlns:wsnt="http://docs.oasis-open.org/wsn/b-2" xmlns:tt="http://www.onvif.org/ver10/schema" xmlns:tns1="http://www.onvif.org/ver10/topics">
<SOAP-ENV:Body>
<tev:PullMessagesResponse>
  <tev:CurrentTime>{}</tev:CurrentTime>
  <tev:TerminationTime>{}</tev:TerminationTime>
  {}
</tev:PullMessagesResponse>
</SOAP-ENV:Body>
</SOAP-ENV:Envelope>"#,
            current_time, termination_time, messages
        )
    }

    fn extract_subscription_url(xml: &str) -> String {
        // Extract the camera's subscription URL from the response
        for prefix in &["wsa5:", "wsa:", ""] {
            let start_tag = format!("<{}Address>", prefix);
            let end_tag = format!("</{}Address>", prefix);

            if let Some(start) = xml.find(&start_tag) {
                if let Some(end) = xml[start..].find(&end_tag) {
                    let url_start = start + start_tag.len();
                    let url_end = start + end;
                    return xml[url_start..url_end].to_string();
                }
            }
        }

        // Fallback to a default if we can't find it
        "/onvif/event_service".to_string()
    }

    fn rewrite_subscription_ref(xml: &str, proxy_url: &str) -> String {
        // Replace the camera's subscription URL with our proxy URL
        let mut result = xml.to_string();

        // Find and replace SubscriptionReference/Address (with or without namespace prefix)
        // Try with namespace prefix first (e.g., <wsa5:Address>, <wsa:Address>)
        for prefix in &["wsa5:", "wsa:", ""] {
            let start_tag = format!("<{}Address>", prefix);
            let end_tag = format!("</{}Address>", prefix);

            if let Some(start) = result.find(&start_tag) {
                if let Some(end) = result[start..].find(&end_tag) {
                    let end_pos = start + end;
                    let before = &result[..start + start_tag.len()];
                    let after = &result[end_pos..];
                    result = format!("{}{}{}", before, proxy_url, after);
                    break;
                }
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
