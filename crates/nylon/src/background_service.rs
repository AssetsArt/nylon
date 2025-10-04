use async_trait::async_trait;
use dashmap::DashMap;
use nylon_types::plugins::FfiPlugin;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use std::{sync::Arc, time::Duration};
use tokio::time::interval;

pub struct NylonBackgroundService;
#[async_trait]
impl BackgroundService for NylonBackgroundService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut period_1d = interval(Duration::from_secs(86400));
        let mut hc_interval = interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    // shutdown
                    tracing::info!("Shutting down background service");

                    // Shutting down plugins
                    let plugins =
                    match nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS) {
                        Some(plugins) => plugins,
                        None => {
                            let new_plugins = DashMap::new();
                            nylon_store::insert(nylon_store::KEY_PLUGINS, new_plugins.clone());
                            new_plugins
                        }
                    };
                    for plugin in plugins.iter() {
                        unsafe {
                            (plugin.value().shutdown)();
                        }
                    }
                    break;
                },
                _ = hc_interval.tick() => {
                    // periodic health checks for all services
                    nylon_store::lb_backends::run_health_checks_for_all().await;
                },
                _ = period_1d.tick() => {
                    tracing::info!("Check certificate expiration");
                }
            }
        }
    }
}
