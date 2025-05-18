use async_trait::async_trait;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use std::time::Duration;
use tokio::time::interval;

pub struct NylonBackgroundService;
#[async_trait]
impl BackgroundService for NylonBackgroundService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut period_1d = interval(Duration::from_secs(86400));
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    // shutdown
                    tracing::info!("Shutting down background service");
                    break;
                },
                _ = period_1d.tick() => {
                    tracing::info!("Check certificate expiration");
                }
            }
        }
    }
}
