use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use nylon_messaging::{NatsClient, NatsClientOptions};
use nylon_types::plugins::{MessagingPhase, PluginPhase};

/// Helper to start an embedded NATS server for testing
/// Requires nats-server binary in PATH or use docker
async fn start_test_nats_server() -> Result<(), Box<dyn std::error::Error>> {
    // For now, we assume NATS server is already running
    // In production tests, we could start a Docker container or embedded server
    Ok(())
}

#[tokio::test]
async fn test_nats_connection() {
    // Start test NATS server
    start_test_nats_server().await.unwrap();

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Connect to NATS
    let options = NatsClientOptions {
        subject_prefix: "nylon.test".to_string(),
        ..Default::default()
    };

    let client = NatsClient::connect(&["nats://localhost:4222"], options)
        .await
        .expect("Failed to connect to NATS");

    // Verify connection is established
    assert!(client.is_connected());

    println!("✅ NATS connection test passed");
}

#[tokio::test]
async fn test_nats_request_reply() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.test".to_string(),
        ..Default::default()
    };

    let client = Arc::new(
        NatsClient::connect(&["nats://localhost:4222"], options)
            .await
            .expect("Failed to connect to NATS"),
    );

    // Setup a responder
    let responder_client = client.clone();
    tokio::spawn(async move {
        let mut sub = responder_client
            .subscribe_queue("nylon.test.echo", "test-workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                let response = format!("echo: {}", String::from_utf8_lossy(&msg.payload));
                responder_client
                    .publish(&reply, response.as_bytes().to_vec())
                    .await
                    .expect("Failed to publish response");
            }
        }
    });

    // Give responder time to start
    sleep(Duration::from_millis(200)).await;

    // Send request
    let response = client
        .request(
            "nylon.test.echo",
            b"hello".to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert_eq!(response_str, "echo: hello");

    println!("✅ NATS request-reply test passed");
}

#[tokio::test]
async fn test_nats_queue_groups() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.test".to_string(),
        ..Default::default()
    };

    let client = Arc::new(
        NatsClient::connect(&["nats://localhost:4222"], options)
            .await
            .expect("Failed to connect to NATS"),
    );

    // Setup multiple workers in same queue group
    let worker_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

    for i in 0..3 {
        let client = client.clone();
        let counter = worker_count.clone();
        tokio::spawn(async move {
            let mut sub = client
                .subscribe_queue("nylon.test.load-balance", "workers")
                .await
                .expect("Failed to subscribe");

            println!("Worker {} ready", i);

            while let Some(msg) = sub.next().await {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if let Some(reply) = msg.reply {
                    let response = format!("worker-{}", i);
                    client
                        .publish(&reply, response.as_bytes().to_vec())
                        .await
                        .expect("Failed to publish");
                }
            }
        });
    }

    // Give workers time to start
    sleep(Duration::from_millis(300)).await;

    // Send multiple requests
    let mut responses = Vec::new();
    for _ in 0..10 {
        let response = client
            .request(
                "nylon.test.load-balance",
                b"test".to_vec(),
                Some(Duration::from_secs(2)),
            )
            .await
            .expect("Failed to send request");

        responses.push(String::from_utf8(response).unwrap());
    }

    // Verify all requests were handled
    assert_eq!(responses.len(), 10);

    // Verify load was distributed (not all handled by same worker)
    let unique_workers: std::collections::HashSet<_> = responses.iter().collect();
    assert!(unique_workers.len() > 1, "Load not distributed across workers");

    println!("✅ NATS queue groups test passed");
    println!("   Requests distributed across {} workers", unique_workers.len());
}

#[tokio::test]
async fn test_nats_timeout_handling() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.test".to_string(),
        ..Default::default()
    };

    let client = NatsClient::connect(&["nats://localhost:4222"], options)
        .await
        .expect("Failed to connect to NATS");

    // Request to non-existent subject should timeout
    let result = client
        .request(
            "nylon.test.nonexistent",
            b"test".to_vec(),
            Some(Duration::from_millis(500)),
        )
        .await;

    assert!(result.is_err(), "Expected timeout error");

    println!("✅ NATS timeout handling test passed");
}

#[tokio::test]
async fn test_plugin_request_filter_flow() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.plugin".to_string(),
        ..Default::default()
    };

    let client = Arc::new(
        NatsClient::connect(&["nats://localhost:4222"], options)
            .await
            .expect("Failed to connect to NATS"),
    );

    // Setup a mock plugin worker
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.request_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            println!("Worker received message");
            
            // Decode PluginRequest (simplified)
            // In real implementation, this would use MessagePack
            
            if let Some(reply) = msg.reply {
                // Send PluginResponse with ResponseAction::Next
                let response = br#"{"version":1,"action":"next"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Simulate Nylon sending a request_filter request
    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.request_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next") || response_str.contains("version"));

    println!("✅ Plugin request_filter flow test passed");
}

#[tokio::test]
async fn test_plugin_error_handling() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.plugin".to_string(),
        ..Default::default()
    };

    let client = Arc::new(
        NatsClient::connect(&["nats://localhost:4222"], options)
            .await
            .expect("Failed to connect to NATS"),
    );

    // Setup a mock plugin worker that returns error
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.error-plugin.request_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                // Send error response
                let response = br#"{"version":1,"action":"error","error":"test error"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send error response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Send request
    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = client
        .request(
            "nylon.plugin.error-plugin.request_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("error"));

    println!("✅ Plugin error handling test passed");
}

#[tokio::test]
async fn test_plugin_multiple_phases() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let options = NatsClientOptions {
        subject_prefix: "nylon.plugin".to_string(),
        ..Default::default()
    };

    let client = Arc::new(
        NatsClient::connect(&["nats://localhost:4222"], options)
            .await
            .expect("Failed to connect to NATS"),
    );

    let phases = vec!["request_filter", "response_filter", "response_body_filter", "logging"];

    for phase in &phases {
        let worker_client = client.clone();
        let phase_name = phase.to_string();
        
        tokio::spawn(async move {
            let subject = format!("nylon.plugin.multi-phase.{}", phase_name);
            let mut sub = worker_client
                .subscribe_queue(&subject, "workers")
                .await
                .expect("Failed to subscribe");

            while let Some(msg) = sub.next().await {
                if let Some(reply) = msg.reply {
                    let response = format!(r#"{{"version":1,"action":"next","phase":"{}"}}"#, phase_name);
                    worker_client
                        .publish(&reply, response.as_bytes().to_vec())
                        .await
                        .expect("Failed to send response");
                }
            }
        });
    }

    sleep(Duration::from_millis(300)).await;

    // Test each phase
    for (idx, phase) in phases.iter().enumerate() {
        let request = format!(r#"{{"version":1,"session_id":1,"phase":{}}}"#, idx + 1);
        
        let response = client
            .request(
                &format!("nylon.plugin.multi-phase.{}", phase),
                request.as_bytes().to_vec(),
                Some(Duration::from_secs(2)),
            )
            .await
            .expect(&format!("Failed to send request for phase {}", phase));

        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("next"));
        
        println!("✅ Phase {} test passed", phase);
    }

    println!("✅ Multiple phases test passed");
}

