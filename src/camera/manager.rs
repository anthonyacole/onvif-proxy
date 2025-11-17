use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::camera::{CameraClient, CameraConfig};

pub struct CameraManager {
    cameras: Arc<RwLock<HashMap<String, CameraClient>>>,
}

impl CameraManager {
    pub fn new() -> Self {
        Self {
            cameras: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_camera(&self, config: CameraConfig) {
        let camera_id = config.id.clone();
        let client = CameraClient::new(config);

        let mut cameras = self.cameras.write().await;
        cameras.insert(camera_id.clone(), client);

        tracing::info!("Added camera: {}", camera_id);
    }

    pub async fn get_camera(&self, camera_id: &str) -> Option<CameraClient> {
        let cameras = self.cameras.read().await;
        cameras.get(camera_id).cloned()
    }
}

impl Clone for CameraManager {
    fn clone(&self) -> Self {
        Self {
            cameras: Arc::clone(&self.cameras),
        }
    }
}

impl Default for CameraManager {
    fn default() -> Self {
        Self::new()
    }
}
