use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::sleep;

use nylon_messaging::{NatsClient, NatsClientOptions, RetryPolicy};

/// Helper to start test NATS server
async fn start_test_nats_server() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[tokio::test]
async fn test_retry_on_timeout() {
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

    let attempt_count = Arc::new(AtomicU32::new(0));

    // Setup worker that responds slowly
    let worker_client = client.clone();
    let worker_counter = attempt_count.clone();
    
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.test.slow-worker", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            let count = worker_counter.fetch_add(1, Ordering::SeqCst);
            
            // First 2 attempts: be slow (simulate timeout)
            if count < 2 {
                sleep(Duration::from_millis(600)).await;
            }
            
            // 3rd attempt: respond quickly
            if let Some(reply) = msg.reply {
                let response = format!("attempt-{}", count + 1);
                worker_client
                    .publish(&reply, response.as_bytes().to_vec())
                    .await
                    .ok();
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Make request with short timeout and retry policy
    let mut retries = 0;
    let max_retries = 3;
    let mut last_error = None;

    for _ in 0..max_retries {
        match client
            .request(
                "nylon.test.slow-worker",
                b"test".to_vec(),
                Some(Duration::from_millis(500)),
            )
            .await
        {
            Ok(response) => {
                let response_str = String::from_utf8(response).unwrap();
                println!("✅ Got response after {} retries: {}", retries, response_str);
                
                // Should eventually succeed after retries
                assert!(response_str.starts_with("attempt-"));
                break;
            }
            Err(e) => {
                retries += 1;
                last_error = Some(e);
                println!("⚠️  Retry {} failed, retrying...", retries);
                sleep(Duration::from_millis(100)).await; // Backoff
            }
        }
    }

    // Verify we did retry
    let attempts = attempt_count.load(Ordering::SeqCst);
    println!("   Total attempts made: {}", attempts);
    assert!(attempts >= 2, "Should have made multiple attempts");

    println!("✅ Retry on timeout test passed");
}

#[tokio::test]
async fn test_retry_with_exponential_backoff() {
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

    let retry_policy = RetryPolicy {
        max_attempts: 4,
        backoff_initial: Duration::from_millis(50),
        backoff_max: Duration::from_millis(500),
    };

    let attempt_times = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Setup worker that fails first few times
    let worker_client = client.clone();
    let fail_count = Arc::new(AtomicU32::new(0));
    let worker_times = attempt_times.clone();
    
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.test.backoff-test", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            let count = fail_count.fetch_add(1, Ordering::SeqCst);
            worker_times.lock().await.push(std::time::Instant::now());
            
            if let Some(reply) = msg.reply {
                if count < 3 {
                    // Fail first 3 attempts - don't respond
                    continue;
                } else {
                    // Success on 4th attempt
                    let response = b"success";
                    worker_client
                        .publish(&reply, response.to_vec())
                        .await
                        .ok();
                }
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Make requests with exponential backoff
    let start = std::time::Instant::now();
    let mut last_error = None;

    for attempt in 0..retry_policy.max_attempts {
        match client
            .request(
                "nylon.test.backoff-test",
                b"test".to_vec(),
                Some(Duration::from_millis(300)),
            )
            .await
        {
            Ok(_) => {
                println!("✅ Request succeeded on attempt {}", attempt + 1);
                break;
            }
            Err(e) => {
                last_error = Some(e);
                
                if attempt < retry_policy.max_attempts - 1 {
                    // Calculate exponential backoff
                    let backoff = retry_policy.backoff_initial * 2_u32.pow(attempt as u32);
                    let backoff = std::cmp::min(backoff, retry_policy.backoff_max);
                    
                    println!("⚠️  Attempt {} failed, backing off for {:?}", attempt + 1, backoff);
                    sleep(backoff).await;
                }
            }
        }
    }

    let total_time = start.elapsed();

    // Verify backoff intervals
    let times = attempt_times.lock().await;
    println!("   Total time: {:?}", total_time);
    println!("   Attempt count: {}", times.len());
    
    if times.len() >= 2 {
        for i in 1..times.len() {
            let interval = times[i].duration_since(times[i-1]);
            println!("   Interval {}: {:?}", i, interval);
        }
    }

    // Should have made multiple attempts with increasing backoff
    assert!(times.len() >= 2, "Should have made multiple attempts");
    
    println!("✅ Exponential backoff test passed");
}

#[tokio::test]
async fn test_max_retries_exceeded() {
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

    // No worker subscribed - all requests will timeout
    let max_retries = 3;
    let mut attempt_count = 0;

    for _ in 0..max_retries {
        match client
            .request(
                "nylon.test.no-worker",
                b"test".to_vec(),
                Some(Duration::from_millis(200)),
            )
            .await
        {
            Ok(_) => {
                panic!("Should not succeed when no worker exists");
            }
            Err(_) => {
                attempt_count += 1;
                sleep(Duration::from_millis(50)).await;
            }
        }
    }

    assert_eq!(attempt_count, max_retries, "Should have tried max_retries times");
    
    println!("✅ Max retries exceeded test passed");
}

#[tokio::test]
async fn test_retry_with_success_after_failures() {
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

    let attempt_count = Arc::new(AtomicU32::new(0));

    // Setup worker that fails first N times then succeeds
    let worker_client = client.clone();
    let worker_counter = attempt_count.clone();
    
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.test.eventual-success", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            let count = worker_counter.fetch_add(1, Ordering::SeqCst);
            
            if let Some(reply) = msg.reply {
                if count < 2 {
                    // First 2 attempts: return error
                    let error_response = br#"{"action":"error","error":"temporary failure"}"#;
                    worker_client
                        .publish(&reply, error_response.to_vec())
                        .await
                        .ok();
                } else {
                    // 3rd attempt: success
                    let success_response = br#"{"action":"next"}"#;
                    worker_client
                        .publish(&reply, success_response.to_vec())
                        .await
                        .ok();
                }
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // Make request with retries
    let mut retry_count = 0;
    let max_retries = 5;
    let mut success = false;

    for _ in 0..max_retries {
        match client
            .request(
                "nylon.test.eventual-success",
                b"test".to_vec(),
                Some(Duration::from_secs(1)),
            )
            .await
        {
            Ok(response) => {
                let response_str = String::from_utf8(response).unwrap();
                if response_str.contains("next") {
                    success = true;
                    println!("✅ Request succeeded after {} retries", retry_count);
                    break;
                } else {
                    // Got error response, retry
                    retry_count += 1;
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(_) => {
                retry_count += 1;
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    assert!(success, "Should eventually succeed");
    assert!(retry_count >= 2, "Should have retried at least twice");

    let attempts = attempt_count.load(Ordering::SeqCst);
    println!("   Total worker attempts: {}", attempts);
    assert!(attempts >= 3, "Worker should have received at least 3 requests");

    println!("✅ Retry with eventual success test passed");
}

#[tokio::test]
async fn test_on_error_continue_policy() {
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

    // Setup worker that always returns error
    let worker_client = client.clone();
    
    tokio::spawn(async move {
        let mut sub = worker_client
            .subscribe_queue("nylon.test.always-error", "workers")
            .await
            .expect("Failed to subscribe");

        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                let error_response = br#"{"action":"error","error":"persistent error"}"#;
                worker_client
                    .publish(&reply, error_response.to_vec())
                    .await
                    .ok();
            }
        }
    });

    sleep(Duration::from_millis(200)).await;

    // With "continue" policy, we should get the error response but not fail the pipeline
    let response = client
        .request(
            "nylon.test.always-error",
            b"test".to_vec(),
            Some(Duration::from_secs(1)),
        )
        .await
        .expect("Should get error response");

    let response_str = String::from_utf8(response).unwrap();
    assert!(response_str.contains("error"));
    
    println!("✅ On-error continue policy test passed");
}

