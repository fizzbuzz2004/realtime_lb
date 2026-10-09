use crate::backend::BackendNode;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct HealthChecker {
    backends: Vec<Arc<BackendNode>>,
    client: Client,
    check_interval: Duration,
}

impl HealthChecker {
    pub fn new(backends: Vec<Arc<BackendNode>>, check_interval: Duration) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("Fehler beim Erstellen des HTTP Health-Check Clients");

        Self {
            backends,
            client,
            check_interval,
        }
    }

    pub async fn start_monitoring(self) {
        info!("Starte Health-Monitor Task...");
        loop {
            for backend in &self.backends {
                let health_url = format!("{}/health", backend.url.trim_end_matches('/'));
                let is_now_healthy = match self.client.get(&health_url).send().await {
                    Ok(resp) => resp.status().is_success(),
                    Err(_) => false,
                };

                let was_healthy = backend.is_healthy();
                if was_healthy != is_now_healthy {
                    backend.set_healthy(is_now_healthy);
                    if is_now_healthy {
                        info!("Backend {} ({}) ist wieder HEALTHY", backend.id, backend.url);
                    } else {
                        warn!("Backend {} ({}) ist UNHEALTHY", backend.id, backend.url);
                    }
                }
            }
            sleep(self.check_interval).await;
        }
    }
}
