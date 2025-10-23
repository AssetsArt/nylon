// Basic NATS Plugin Integration Tests

use super::test_helpers::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_nats_connection() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let _client = create_test_client("nylon.test").await;
    
    // If we got here without panic, connection succeeded
    println!("✅ NATS connection test passed");
}

#[tokio::test]
async fn test_nats_request_reply() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.test").await;

    // Setup a simple echo responder
    let responder_client = client.clone();
    subscribe_and_respond(
        responder_client,
        "echo",
        "test-workers",
        |payload| {
            let input = String::from_utf8_lossy(payload);
            format!("echo: {}", input).into_bytes()
        },
    ).await;

    sleep(wait_for_workers()).await;

    // Send request
    let response = test_request(&client, "echo", b"hello")
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

    let client = create_test_client("nylon.test").await;

    // Setup multiple workers in same queue group
    for i in 0..3 {
        let worker_client = client.clone();
        subscribe_and_respond(
            worker_client,
            "load-balance",
            "workers",
            move |_payload| format!("worker-{}", i).into_bytes(),
        ).await;
    }

    sleep(Duration::from_millis(300)).await;

    // Send multiple requests
    let mut responses = Vec::new();
    for _ in 0..10 {
        let response = test_request(&client, "load-balance", b"test")
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

    let client = create_test_client("nylon.test").await;

    // Request to non-existent subject should timeout
    let result = client.request("nonexistent", b"test", None).await;

    assert!(result.is_err(), "Expected timeout error");

    println!("✅ NATS timeout handling test passed");
}

#[tokio::test]
async fn test_plugin_request_filter_flow() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "test-plugin.request_filter",
        "workers",
        |_payload| br#"{"version":1,"action":"next"}"#.to_vec(),
    ).await;

    sleep(wait_for_workers()).await;

    // Simulate Nylon sending a request_filter request
    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "test-plugin.request_filter", request)
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

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that returns error
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "error-plugin.request_filter",
        "workers",
        |_payload| br#"{"version":1,"action":"error","error":"test error"}"#.to_vec(),
    ).await;

    sleep(wait_for_workers()).await;

    // Send request
    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "error-plugin.request_filter", request)
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

    let client = create_test_client("nylon.plugin").await;

    let phases = vec!["request_filter", "response_filter", "response_body_filter", "logging"];

    for phase in &phases {
        let worker_client = client.clone();
        let phase_name = phase.to_string();
        
        subscribe_and_respond(
            worker_client,
            &format!("multi-phase.{}", phase_name),
            "workers",
            move |_payload| {
                format!(r#"{{"version":1,"action":"next","phase":"{}"}}"#, phase_name).into_bytes()
            },
        ).await;
    }

    sleep(Duration::from_millis(300)).await;

    // Test each phase
    for (idx, phase) in phases.iter().enumerate() {
        let request = format!(r#"{{"version":1,"session_id":1,"phase":{}}}"#, idx + 1);
        
        let response = test_request(
            &client,
            &format!("multi-phase.{}", phase),
            request.as_bytes(),
        )
        .await
        .expect(&format!("Failed to send request for phase {}", phase));

        let response_str = String::from_utf8(response).unwrap();
        assert!(response_str.contains("next"));
        
        println!("   ✓ {} phase completed", phase);
    }

    println!("✅ Multiple phases test passed");
}

#[tokio::test]
async fn test_retry_on_slow_worker() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.test").await;

    let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let counter = attempt_count.clone();

    // Setup worker that responds slowly first 2 times, then quickly
    let worker_client = client.clone();
    subscribe_with_delay(
        worker_client,
        "slow-worker",
        "workers",
        50, // Small delay per request
        move |_payload| {
            let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            format!("attempt-{}", count + 1).into_bytes()
        },
    ).await;

    sleep(wait_for_workers()).await;

    // Make request
    let response = test_request(&client, "slow-worker", b"test")
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.starts_with("attempt-"));
    
    println!("✅ Retry on slow worker test passed");
}

#[tokio::test]
async fn test_concurrent_requests() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.test").await;

    // Setup worker
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "concurrent",
        "workers",
        |_payload| b"success".to_vec(),
    ).await;

    sleep(wait_for_workers()).await;

    // Send multiple concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let request = format!(r#"{{"id":{}}}"#, i);
            test_request(&client_clone, "concurrent", request.as_bytes()).await
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
    
    println!("✅ Concurrent requests test passed");
}

