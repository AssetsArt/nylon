use crate::{
    error::{ErrorKind, MessagingError},
    protocol::MessageHeaders,
};
use async_nats::{Client, Subscriber, header::HeaderMap};
use nylon_types::plugins::OverflowPolicy;
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{self, Instant};
use tracing::{debug, warn};

pub type QueueSubscription = Subscriber;

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_attempts: usize,
    pub backoff_initial: Duration,
    pub backoff_max: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff_initial: Duration::from_millis(50),
            backoff_max: Duration::from_millis(250),
        }
    }
}

impl RetryPolicy {
    pub fn should_retry(&self, attempt: usize, error: &MessagingError) -> bool {
        if attempt >= self.max_attempts {
            return false;
        }

        matches!(
            error,
            MessagingError::Timeout { .. } | MessagingError::Request(_) | MessagingError::Closed
        )
    }

    pub fn backoff_delay(&self, attempt: usize) -> Duration {
        if attempt == 0 {
            return self.backoff_initial;
        }

        let capped = attempt.saturating_sub(1).min(16) as u32;
        let multiplier = 1u64.checked_shl(capped).unwrap_or(u64::MAX);
        let min_delay = self.backoff_initial.as_millis() as u64;
        let max_delay = self.backoff_max.as_millis() as u64;
        let calc = min_delay
            .saturating_mul(multiplier)
            .clamp(min_delay, max_delay);
        Duration::from_millis(calc)
    }
}

#[derive(Clone, Debug)]
pub struct NatsClientOptions {
    pub servers: Vec<String>,
    pub name: Option<String>,
    pub subject_prefix: Option<String>,
    pub request_timeout: Duration,
    pub retry_policy: RetryPolicy,
    pub max_inflight: usize,
    pub overflow_policy: OverflowPolicy,
    pub default_headers: MessageHeaders,
}

impl NatsClientOptions {
    pub fn new<T: Into<Vec<String>>>(servers: T) -> Self {
        Self {
            servers: servers.into(),
            name: None,
            subject_prefix: None,
            request_timeout: Duration::from_millis(500),
            retry_policy: RetryPolicy::default(),
            max_inflight: 1024,
            overflow_policy: OverflowPolicy::default(),
            default_headers: MessageHeaders::default(),
        }
    }
}

impl Default for NatsClientOptions {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[derive(Clone)]
pub struct NatsClient {
    client: Client,
    request_timeout: Duration,
    retry_policy: RetryPolicy,
    overflow_policy: OverflowPolicy,
    inflight: Option<Arc<Semaphore>>,
    subject_prefix: Option<String>,
    default_headers: MessageHeaders,
}

impl NatsClient {
    pub async fn connect(options: NatsClientOptions) -> Result<Self, MessagingError> {
        if options.servers.is_empty() {
            return Err(MessagingError::Connect(
                "at least one NATS server must be provided".to_string(),
            ));
        }

        let server = options.servers.first().cloned().ok_or_else(|| {
            MessagingError::Connect("at least one NATS server must be provided".to_string())
        })?;

        let client = async_nats::connect(server)
            .await
            .map_err(|err| MessagingError::from_error(ErrorKind::Connect, err))?;

        let inflight = if options.max_inflight == 0 {
            None
        } else {
            Some(Arc::new(Semaphore::new(options.max_inflight)))
        };

        Ok(Self {
            client,
            request_timeout: options.request_timeout,
            retry_policy: options.retry_policy,
            overflow_policy: options.overflow_policy,
            inflight,
            subject_prefix: options.subject_prefix,
            default_headers: options.default_headers,
        })
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub fn subject_prefix(&self) -> Option<&str> {
        self.subject_prefix.as_deref()
    }

    fn expand_subject<'a>(&self, subject: &'a str) -> String {
        match &self.subject_prefix {
            Some(prefix) if !subject.starts_with(prefix) => {
                let prefix = prefix.trim_end_matches('.');
                let subject = subject.trim_start_matches('.');
                format!("{prefix}.{subject}")
            }
            _ => subject.to_string(),
        }
    }

    fn merge_headers(&self, extra: Option<&MessageHeaders>) -> Option<HeaderMap> {
        if self.default_headers.is_empty() && extra.map(|h| h.is_empty()).unwrap_or(true) {
            return None;
        }

        let mut headers = HeaderMap::new();
        for (key, value) in self.default_headers.iter() {
            headers.insert(key.as_str(), value.as_str());
        }
        if let Some(extra) = extra {
            for (key, value) in extra.iter() {
                headers.insert(key.as_str(), value.as_str());
            }
        }
        Some(headers)
    }

    async fn acquire_permit(&self) -> Result<Option<OwnedSemaphorePermit>, MessagingError> {
        let Some(semaphore) = self.inflight.as_ref() else {
            return Ok(None);
        };

        match self.overflow_policy {
            OverflowPolicy::Queue => Ok(Some(
                semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| MessagingError::Closed)?,
            )),
            OverflowPolicy::Reject | OverflowPolicy::Shed => semaphore
                .clone()
                .try_acquire_owned()
                .map(Some)
                .map_err(|_| MessagingError::Overflow {
                    policy: self.overflow_policy,
                }),
        }
    }

    pub async fn request(
        &self,
        subject: &str,
        payload: &[u8],
        headers: Option<&MessageHeaders>,
    ) -> Result<Vec<u8>, MessagingError> {
        let subject = self.expand_subject(subject);
        let permit = self.acquire_permit().await?;
        let mut attempt = 0usize;

        loop {
            attempt += 1;
            let attempt_start = Instant::now();
            let result = self.request_once(&subject, payload, headers).await;

            match result {
                Ok(bytes) => {
                    drop(permit);
                    debug!(subject = %subject, attempt, elapsed_ms = attempt_start.elapsed().as_millis(), "NATS request succeeded");
                    return Ok(bytes);
                }
                Err(err) => {
                    if !self.retry_policy.should_retry(attempt, &err) {
                        drop(permit);
                        return Err(err);
                    }

                    let delay = self.retry_policy.backoff_delay(attempt);
                    warn!(subject = %subject, attempt, delay_ms = delay.as_millis(), error = %err, "retrying NATS request");
                    time::sleep(delay).await;
                }
            }
        }
    }

    async fn request_once(
        &self,
        subject: &str,
        payload: &[u8],
        headers: Option<&MessageHeaders>,
    ) -> Result<Vec<u8>, MessagingError> {
        let merged_headers = self.merge_headers(headers);
        let client = self.client.clone();
        let payload_bytes = payload.to_vec();
        let subject_owned = subject.to_string();
        let subject_for_request = subject_owned.clone();

        let fut = async move {
            match merged_headers {
                Some(map) => {
                    let payload_clone = payload_bytes.clone();
                    client
                        .request_with_headers(subject_owned.clone(), map, payload_clone.into())
                        .await
                }
                None => {
                    client
                        .request(subject_for_request, payload_bytes.into())
                        .await
                }
            }
        };

        let message = time::timeout(self.request_timeout, fut)
            .await
            .map_err(|_| MessagingError::Timeout {
                timeout: self.request_timeout,
            })
            .and_then(|res| {
                res.map_err(|err| MessagingError::from_error(ErrorKind::Request, err))
            })?;

        Ok(message.payload.to_vec())
    }

    pub async fn publish(
        &self,
        subject: &str,
        payload: &[u8],
        headers: Option<&MessageHeaders>,
    ) -> Result<(), MessagingError> {
        let subject = self.expand_subject(subject);
        let merged_headers = self.merge_headers(headers);
        let payload_bytes = payload.to_vec();

        if let Some(map) = merged_headers {
            self.client
                .publish_with_headers(subject.clone(), map, payload_bytes.clone().into())
                .await
                .map_err(|err| MessagingError::from_error(ErrorKind::Publish, err))?;
        } else {
            self.client
                .publish(subject.clone(), payload_bytes.into())
                .await
                .map_err(|err| MessagingError::from_error(ErrorKind::Publish, err))?;
        }

        Ok(())
    }

    pub async fn subscribe_queue(
        &self,
        subject: &str,
        queue_group: &str,
    ) -> Result<QueueSubscription, MessagingError> {
        let subject = self.expand_subject(subject);
        self.client
            .queue_subscribe(subject, queue_group.to_string())
            .await
            .map_err(|err| MessagingError::from_error(ErrorKind::Subscription, err))
    }
}
