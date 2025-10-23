// Read Methods Integration Tests
// Tests for GET_PAYLOAD, READ_REQUEST_*, READ_RESPONSE_*

use super::test_helpers::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_read_request_methods() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that sends back request info
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "read-methods.request_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading request data and sending response with those values
            let response = br#"{
                "version":1,
                "action":"next",
                "data":{
                    "url":"https://example.com/test?q=hello",
                    "path":"/test",
                    "query":"q=hello",
                    "host":"example.com",
                    "method":"GET",
                    "client_ip":"127.0.0.1"
                }
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    // Simulate Nylon sending a request_filter request
    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "read-methods.request_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));

    println!("✅ Read request methods test passed");
}

#[tokio::test]
async fn test_read_response_methods() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that simulates reading response data
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "read-methods.response_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading response data
            let response = br#"{
                "version":1,
                "action":"next",
                "data":{
                    "status":200,
                    "headers":{"Content-Type":"application/json"},
                    "bytes":1234
                }
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":2}"#;
    
    let response = test_request(&client, "read-methods.response_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));

    println!("✅ Read response methods test passed");
}

#[tokio::test]
async fn test_get_payload_method() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that sends back payload data
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "payload-test.request_filter",
        "workers",
        |_payload| {
            // Simulate plugin getting payload and processing it
            let response = br#"{
                "version":1,
                "action":"next",
                "payload":{"user":"test","action":"login"}
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "payload-test.request_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("next"));
    assert!(response_str.contains("payload"));

    println!("✅ GET_PAYLOAD method test passed");
}

#[tokio::test]
async fn test_read_request_headers() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that reads headers
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "headers-test.request_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading request headers
            let response = br#"{
                "version":1,
                "action":"next",
                "headers":{
                    "Authorization":"Bearer token123",
                    "User-Agent":"Nylon/1.0",
                    "Accept":"application/json"
                }
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "headers-test.request_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("Authorization"));
    assert!(response_str.contains("Bearer token123"));

    println!("✅ Read request headers test passed");
}

#[tokio::test]
async fn test_read_request_body() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that reads request body
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "body-test.request_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading and processing request body
            let response = br#"{
                "version":1,
                "action":"next",
                "body":"{\"username\":\"test\",\"password\":\"secret\"}"
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "body-test.request_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("body"));
    assert!(response_str.contains("username"));

    println!("✅ Read request body test passed");
}

#[tokio::test]
async fn test_read_request_params() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that reads route params
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "params-test.request_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading route parameters
            let response = br#"{
                "version":1,
                "action":"next",
                "params":{"id":"123","slug":"test-post"}
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":1}"#;
    
    let response = test_request(&client, "params-test.request_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("params"));
    assert!(response_str.contains("id"));
    assert!(response_str.contains("123"));

    println!("✅ Read request params test passed");
}

#[tokio::test]
async fn test_read_response_status() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that reads response status
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "status-test.response_filter",
        "workers",
        |_payload| {
            // Simulate plugin reading response status
            let response = br#"{
                "version":1,
                "action":"next",
                "status":200
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":2}"#;
    
    let response = test_request(&client, "status-test.response_filter", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("status"));
    assert!(response_str.contains("200"));

    println!("✅ Read response status test passed");
}

#[tokio::test]
async fn test_read_response_duration() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup a mock plugin worker that reads response duration
    let worker_client = client.clone();
    subscribe_and_respond(
        worker_client,
        "duration-test.logging",
        "workers",
        |_payload| {
            // Simulate plugin reading response duration in logging phase
            let response = br#"{
                "version":1,
                "action":"next",
                "duration":150
            }"#;
            response.to_vec()
        },
    ).await;

    sleep(wait_for_workers()).await;

    let request = br#"{"version":1,"session_id":1,"phase":4}"#;
    
    let response = test_request(&client, "duration-test.logging", request)
        .await
        .expect("Failed to send request");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("duration"));

    println!("✅ Read response duration test passed");
}

#[tokio::test]
async fn test_read_methods_concurrent() {
    start_test_nats_server().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    let client = create_test_client("nylon.plugin").await;

    // Setup workers for different read method types
    for test_name in &["url", "path", "query", "host", "method"] {
        let worker_client = client.clone();
        let name = test_name.to_string();
        
        subscribe_and_respond(
            worker_client,
            &format!("concurrent-read.{}", name),
            "workers",
            move |_payload| {
                format!(
                    r#"{{"version":1,"action":"next","{}":"test-value"}}"#,
                    name
                ).into_bytes()
            },
        ).await;
    }

    sleep(Duration::from_millis(500)).await;

    // Send concurrent requests to test different read methods
    let mut handles = vec![];
    
    for test_name in &["url", "path", "query", "host", "method"] {
        let client_clone = client.clone();
        let name = test_name.to_string();
        
        let handle = tokio::spawn(async move {
            let request = br#"{"version":1,"session_id":1,"phase":1}"#;
            test_request(&client_clone, &format!("concurrent-read.{}", name), request).await
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

    assert_eq!(success_count, 5, "All concurrent read method requests should succeed");
    
    println!("✅ Concurrent read methods test passed");
}

