// Test helper functions for NATS integration tests

use futures::StreamExt;
use nylon_messaging::{NatsClient, NatsClientOptions};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

pub async fn create_test_client(prefix: &str) -> Arc<NatsClient> {
    let mut options = NatsClientOptions::new(vec!["nats://localhost:4222".to_string()]);
    options.subject_prefix = Some(prefix.to_string());
    options.request_timeout = Duration::from_secs(5);
    
    Arc::new(
        NatsClient::connect(options)
            .await
            .expect("Failed to connect to NATS"),
    )
}

pub async fn test_request(
    client: &NatsClient,
    subject: &str,
    payload: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let response = client.request(subject, payload, None).await?;
    Ok(response)
}

pub async fn test_publish(
    client: &NatsClient,
    subject: &str,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    client.publish(subject, payload, None).await?;
    Ok(())
}

pub async fn subscribe_and_respond<F>(
    client: Arc<NatsClient>,
    subject: &str,
    queue_group: &str,
    handler: F,
) where
    F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    let subject = subject.to_string();
    let queue_group = queue_group.to_string();
    let handler = Arc::new(handler);
    
    tokio::spawn(async move {
        let mut sub = client
            .subscribe_queue(&subject, &queue_group)
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            let response = handler(&msg.payload);
            
            if let Some(reply) = msg.reply.as_ref() {
                // Use raw NATS client for reply to avoid subject prefix expansion
                let raw_client = client.client();
                let _result = raw_client
                    .publish(reply.clone(), response.into())
                    .await;
            }
        }
    });
}

pub async fn subscribe_with_delay<F>(
    client: Arc<NatsClient>,
    subject: &str,
    queue_group: &str,
    delay_ms: u64,
    handler: F,
) where
    F: Fn(&[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    let subject = subject.to_string();
    let queue_group = queue_group.to_string();
    let handler = Arc::new(handler);
    
    tokio::spawn(async move {
        let mut sub = client
            .subscribe_queue(&subject, &queue_group)
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let response = handler(&msg.payload);
            
            if let Some(reply) = msg.reply.as_ref() {
                // Use raw NATS client for reply to avoid subject prefix expansion
                let raw_client = client.client();
                let _result = raw_client
                    .publish(reply.clone(), response.into())
                    .await;
            }
        }
    });
}

pub async fn start_test_nats_server() -> Result<(), Box<dyn std::error::Error>> {
    // Assumes NATS server is already running
    // In CI/CD, this would start a Docker container or embedded server
    Ok(())
}

pub fn wait_for_workers() -> Duration {
    Duration::from_millis(500)
}

pub fn short_timeout() -> Duration {
    Duration::from_millis(500)
}

pub fn medium_timeout() -> Duration {
    Duration::from_secs(2)
}

pub fn long_timeout() -> Duration {
    Duration::from_secs(5)
}

