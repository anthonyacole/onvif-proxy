use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub proxy: ProxyConfig,
    pub cameras: Vec<CameraConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProxyConfig {
    pub listen_address: String,
    pub base_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CameraConfig {
    pub id: String,
    pub name: String,
    pub address: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub enable_smart_detection: bool,
    #[serde(default)]
    pub quirks: Vec<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_model() -> String {
    "reolink".to_string()
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .context("Failed to read configuration file")?;

        let config: AppConfig = serde_yaml::from_str(&contents)
            .context("Failed to parse YAML configuration")?;

        Ok(config)
    }

    pub fn get_camera(&self, camera_id: &str) -> Option<&CameraConfig> {
        self.cameras.iter().find(|c| c.id == camera_id)
    }
}

impl CameraConfig {
    pub fn base_url(&self) -> String {
        format!("http://{}", self.address)
    }
}
