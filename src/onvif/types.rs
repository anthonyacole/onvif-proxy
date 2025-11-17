use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInformation {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub analytics: Option<AnalyticsCapabilities>,
    pub device: Option<DeviceCapabilities>,
    pub events: Option<EventsCapabilities>,
    pub imaging: Option<ImagingCapabilities>,
    pub media: Option<MediaCapabilities>,
    pub ptz: Option<PtzCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsCapabilities {
    pub xaddr: String,
    pub rule_support: bool,
    pub analytics_module_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub xaddr: String,
    pub network: NetworkCapabilities,
    pub system: SystemCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    pub ip_filter: bool,
    pub zero_configuration: bool,
    pub ip_version6: bool,
    pub dyn_dns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    pub discovery_resolve: bool,
    pub discovery_bye: bool,
    pub remote_discovery: bool,
    pub system_backup: bool,
    pub system_logging: bool,
    pub firmware_upgrade: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsCapabilities {
    pub xaddr: String,
    pub ws_subscription_policy_support: bool,
    pub ws_pull_point_support: bool,
    pub ws_pausable_subscription_manager_interface_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagingCapabilities {
    pub xaddr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaCapabilities {
    pub xaddr: String,
    pub streaming_capabilities: StreamingCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingCapabilities {
    pub rtp_multicast: bool,
    pub rtp_tcp: bool,
    pub rtp_rtsp_tcp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzCapabilities {
    pub xaddr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub token: String,
    pub name: String,
    pub video_source_configuration: Option<VideoSourceConfiguration>,
    pub video_encoder_configuration: Option<VideoEncoderConfiguration>,
    pub ptz_configuration: Option<PtzConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSourceConfiguration {
    pub token: String,
    pub name: String,
    pub source_token: String,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoderConfiguration {
    pub token: String,
    pub name: String,
    pub encoding: String,
    pub resolution: Resolution,
    pub quality: f32,
    pub rate_control: RateControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateControl {
    pub framerate_limit: i32,
    pub encoding_interval: i32,
    pub bitrate_limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtzConfiguration {
    pub token: String,
    pub name: String,
    pub node_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUri {
    pub uri: String,
    pub invalid_after_connect: bool,
    pub invalid_after_reboot: bool,
    pub timeout: String,
}
