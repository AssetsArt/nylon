use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use nylon_messaging::{NatsClient, NatsClientOptions};

/// Helper to start test NATS server
async fn start_test_nats_server() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[tokio::test]
async fn test_response_filter_basic() {
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

    // Setup mock worker for response_filter phase
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.response_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            println!("ResponseFilter worker received message");
            
            if let Some(reply) = msg.reply {
                // Simulate modifying response headers
                let response = br#"{"version":1,"action":"next","headers":{"X-Modified":"true"}}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Simulate Nylon sending response_filter request (phase=2)
    let request = br#"{"version":1,"session_id":1,"phase":2}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.response_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));
    
    println!("✅ Response filter basic test passed");
}

#[tokio::test]
async fn test_response_body_filter() {
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

    // Setup mock worker for response_body_filter phase
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.response_body_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            println!("ResponseBodyFilter worker received message");
            
            if let Some(reply) = msg.reply {
                // Simulate modifying response body
                let response = br#"{"version":1,"action":"next","body":"modified body"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Simulate Nylon sending response_body_filter request (phase=3)
    let request = br#"{"version":1,"session_id":1,"phase":3}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.response_body_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));
    
    println!("✅ Response body filter test passed");
}

#[tokio::test]
async fn test_logging_phase() {
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

    // Setup mock worker for logging phase
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.logging", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            println!("Logging worker received message");
            
            if let Some(reply) = msg.reply {
                // Logging phase typically just returns next
                let response = br#"{"version":1,"action":"next"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Simulate Nylon sending logging request (phase=4)
    let request = br#"{"version":1,"session_id":1,"phase":4}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.logging",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));
    
    println!("✅ Logging phase test passed");
}

#[tokio::test]
async fn test_response_filter_end_action() {
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

    // Setup mock worker that returns "end" action
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.response_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                // Return "end" action to stop pipeline
                let response = br#"{"version":1,"action":"end"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    let request = br#"{"version":1,"session_id":1,"phase":2}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.response_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("end"));
    
    println!("✅ Response filter end action test passed");
}

#[tokio::test]
async fn test_response_filter_with_headers() {
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

    // Setup mock worker that modifies multiple headers
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.test-plugin.response_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                let response = br#"{"version":1,"action":"next","headers":{"X-Custom":"value","X-Another":"header"}}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .expect("Failed to send response");
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    let request = br#"{"version":1,"session_id":1,"phase":2}"#;
    
    let response = client
        .request(
            "nylon.plugin.test-plugin.response_filter",
            request.to_vec(),
            Some(Duration::from_secs(2)),
        )
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("X-Custom"));
    assert!(response_str.contains("X-Another"));
    
    println!("✅ Response filter with headers test passed");
}

#[tokio::test]
async fn test_full_pipeline_simulation() {
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

    // Setup workers for all phases
    let phases = vec![
        ("request_filter", 1),
        ("response_filter", 2),
        ("response_body_filter", 3),
        ("logging", 4),
    ];

    for (phase_name, _phase_id) in &phases {
        let worker_client = client.clone();
        let phase = phase_name.to_string();
        
        tokio::spawn(async move {
            let subject = format!("nylon.plugin.full-pipeline.{}", phase);
            let mut sub = worker_client
                .subscribe_queue(&subject, "workers")
                .await
                .expect("Failed to subscribe");

            while let Some(msg) = sub.next().await {
                if let Some(reply) = msg.reply {
                    let response = format!(
                        r#"{{"version":1,"action":"next","phase":"{}"}}"#,
                        phase
                    );
                    worker_client
                        .publish(&reply, response.as_bytes().to_vec())
                        .await
                        .expect("Failed to send response");
                }
            }
        });
    }

    sleep(Duration::from_millis(300)).await;

    // Simulate full pipeline: request -> response -> body -> logging
    for (phase_name, phase_id) in &phases {
        let request = format!(
            r#"{{"version":1,"session_id":1,"phase":{}}}"#,
            phase_id
        );
        
        let response = client
            .request(
                &format!("nylon.plugin.full-pipeline.{}", phase_name),
                request.as_bytes().to_vec(),
                Some(Duration::from_secs(2)),
            )
            .await
            .expect(&format!("Failed in {} phase", phase_name));

        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("next"));
        
        println!("   ✓ {} phase completed", phase_name);
    }

    println!("✅ Full pipeline simulation test passed");
}

#[tokio::test]
async fn test_concurrent_response_filters() {
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

    // Setup worker
    let worker_client = client.clone();
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.plugin.concurrent.response_filter", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            // Simulate some processing time
            sleep(Duration::from_millis(50)).await;
            
            if let Some(reply) = msg.reply {
                let response = br#"{"version":1,"action":"next"}"#;
                worker_client
                    .publish(&reply, response.to_vec())
                    .await
                    .ok();
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Send multiple concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = format!(r#"{{"version":1,"session_id":{},"phase":2}}"#, i);
            
            client_clone
                .request(
                    "nylon.plugin.concurrent.response_filter",
                    request.as_bytes().to_vec(),
                    Some(Duration::from_secs(2)),
                )
                .await
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(_)) = handle.await {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 10, "All concurrent requests should succeed");
    
    println!("✅ Concurrent response filters test passed");
}

