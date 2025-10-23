use bytes::Bytes;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_messaging::{
    MessageHeaders, MessagingError, MessagingTransport, NatsClient, NatsClientOptions, RetryPolicy, new_request_id,
};
use nylon_store::{self, KEY_MESSAGING_CONFIG, KEY_MESSAGING_PLUGINS};
use nylon_types::context::NylonContext;
use nylon_types::plugins::{
    MessagingAuthConfig, MessagingConfig, MessagingOnError, MessagingPhase, MessagingPhaseConfig,
    MessagingTlsConfig, OverflowPolicy, PluginItem, PluginPhase, RetryPolicyConfig,
};
use nylon_types::template::Expr;
use nylon_types::transport::TraceMeta;
use once_cell::sync::OnceCell;
use pingora::proxy::{ProxyHttp, Session};
use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tracing::debug;

use crate::transport_handler::TransportSessionHandler;

#[derive(Clone, Debug)]
pub struct PhasePolicy {
    pub timeout: Option<Duration>,
    pub on_error: MessagingOnError,
    pub retry: RetryPolicy,
}

impl Default for PhasePolicy {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_millis(5000)),
            on_error: MessagingOnError::Retry,
            retry: RetryPolicy::default(),
        }
    }
}

impl PhasePolicy {
    /// Default policy for request_filter phase
    pub fn request_filter_default() -> Self {
        Self {
            timeout: Some(Duration::from_millis(5000)),
            on_error: MessagingOnError::Retry,
            retry: RetryPolicy {
                max_attempts: 3,
                backoff_initial: Duration::from_millis(100),
                backoff_max: Duration::from_millis(1000),
            },
        }
    }

    /// Default policy for response_filter phase
    pub fn response_filter_default() -> Self {
        Self {
            timeout: Some(Duration::from_millis(3000)),
            on_error: MessagingOnError::Continue,
            retry: RetryPolicy {
                max_attempts: 2,
                backoff_initial: Duration::from_millis(50),
                backoff_max: Duration::from_millis(500),
            },
        }
    }

    /// Default policy for logging phase
    pub fn logging_default() -> Self {
        Self {
            timeout: Some(Duration::from_millis(200)),
            on_error: MessagingOnError::Continue,
            retry: RetryPolicy {
                max_attempts: 1,
                backoff_initial: Duration::from_millis(0),
                backoff_max: Duration::from_millis(0),
            },
        }
    }

    /// Get default policy for a specific phase
    pub fn for_phase(phase: MessagingPhase) -> Self {
        match phase {
            MessagingPhase::RequestFilter => Self::request_filter_default(),
            MessagingPhase::ResponseFilter => Self::response_filter_default(),
            MessagingPhase::ResponseBodyFilter => Self::response_filter_default(),
            MessagingPhase::Logging => Self::logging_default(),
        }
    }
}

pub struct MessagingPlugin {
    plugin_name: String,
    config_name: String,
    queue_group: String,
    options: NatsClientOptions,
    per_phase: HashMap<MessagingPhase, PhasePolicy>,
    auth: Option<MessagingAuthConfig>,
    tls: Option<MessagingTlsConfig>,
    client: OnceCell<Arc<NatsClient>>,
    connect_lock: Mutex<()>,
}

impl fmt::Debug for MessagingPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MessagingPlugin")
            .field("plugin_name", &self.plugin_name)
            .field("config_name", &self.config_name)
            .field("queue_group", &self.queue_group)
            .finish()
    }
}

static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);

impl MessagingPlugin {
    pub fn new(plugin: &PluginItem, config: &MessagingConfig) -> Result<Self, NylonError> {
        if config.servers.is_empty() {
            return Err(NylonError::ConfigError(format!(
                "Messaging config '{}' must specify at least one server",
                config.name
            )));
        }

        let mut options = NatsClientOptions::new(config.servers.clone());
        options.subject_prefix = config.subject_prefix.clone();
        options.request_timeout = Duration::from_millis(config.request_timeout_ms.unwrap_or(500));

        let max_inflight = plugin.max_inflight.or(config.max_inflight).unwrap_or(1024) as usize;
        options.max_inflight = max_inflight;

        let overflow_policy: OverflowPolicy = plugin
            .overflow_policy
            .or(config.overflow_policy)
            .unwrap_or_default();
        options.overflow_policy = overflow_policy;

        options.retry_policy = merge_retry_policy(config.retry.as_ref(), None);

        options.default_headers = config
            .default_headers
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect::<MessageHeaders>();

        let per_phase = build_phase_policies(plugin.per_phase.as_ref(), config.retry.as_ref());

        let queue_group = plugin
            .group
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "default".to_string());

        Ok(Self {
            plugin_name: plugin.name.clone(),
            config_name: config.name.clone(),
            queue_group,
            options,
            per_phase,
            auth: config.auth.clone(),
            tls: config.tls.clone(),
            client: OnceCell::new(),
            connect_lock: Mutex::new(()),
        })
    }

    pub fn plugin_name(&self) -> &str {
        &self.plugin_name
    }

    pub fn queue_group(&self) -> &str {
        &self.queue_group
    }

    pub fn config_name(&self) -> &str {
        &self.config_name
    }

    pub fn options(&self) -> &NatsClientOptions {
        &self.options
    }

    pub fn per_phase(&self) -> &HashMap<MessagingPhase, PhasePolicy> {
        &self.per_phase
    }

    pub fn auth(&self) -> Option<&MessagingAuthConfig> {
        self.auth.as_ref()
    }

    pub fn tls(&self) -> Option<&MessagingTlsConfig> {
        self.tls.as_ref()
    }

    pub async fn client(&self) -> Result<Arc<NatsClient>, MessagingError> {
        if let Some(client) = self.client.get() {
            return Ok(client.clone());
        }

        let _guard = self.connect_lock.lock().await;
        if let Some(client) = self.client.get() {
            return Ok(client.clone());
        }

        let client = NatsClient::connect(self.options.clone()).await?;
        let arc = Arc::new(client);
        let _ = self.client.set(arc.clone());
        Ok(arc)
    }
}

fn next_session_id() -> u32 {
    NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed)
}

fn build_phase_policies(
    per_phase: Option<&HashMap<MessagingPhase, MessagingPhaseConfig>>,
    base_retry: Option<&RetryPolicyConfig>,
) -> HashMap<MessagingPhase, PhasePolicy> {
    use MessagingPhase::*;
    let mut policies = HashMap::new();

    // Start with sensible defaults for all phases
    for phase in [RequestFilter, ResponseFilter, ResponseBodyFilter, Logging] {
        policies.insert(phase, PhasePolicy::for_phase(phase));
    }

    // Override with user config if provided
    if let Some(configs) = per_phase {
        for (phase, cfg) in configs {
            let default_policy = PhasePolicy::for_phase(*phase);
            
            let timeout = cfg.timeout_ms
                .map(Duration::from_millis)
                .or(default_policy.timeout);
            
            let on_error = cfg.on_error.unwrap_or(default_policy.on_error);
            
            let retry = if cfg.retry.is_some() || base_retry.is_some() {
                merge_retry_policy(base_retry, cfg.retry.as_ref())
            } else {
                default_policy.retry
            };
            
            policies.insert(
                *phase,
                PhasePolicy {
                    timeout,
                    on_error,
                    retry,
                },
            );
        }
    }

    policies
}

fn merge_retry_policy(
    base: Option<&RetryPolicyConfig>,
    override_cfg: Option<&RetryPolicyConfig>,
) -> RetryPolicy {
    let mut policy = RetryPolicy::default();

    if let Some(config) = base {
        apply_retry_config(&mut policy, config);
    }
    if let Some(config) = override_cfg {
        apply_retry_config(&mut policy, config);
    }

    policy
}

fn apply_retry_config(policy: &mut RetryPolicy, config: &RetryPolicyConfig) {
    if let Some(max) = config.max {
        if max > 0 {
            policy.max_attempts = max as usize;
        }
    }
    if let Some(initial) = config.backoff_ms_initial {
        if initial > 0 {
            policy.backoff_initial = Duration::from_millis(initial);
        }
    }
    if let Some(max) = config.backoff_ms_max {
        if max > 0 {
            policy.backoff_max = Duration::from_millis(max);
        }
    }
}

fn phase_subject_fragment(phase: PluginPhase) -> &'static str {
    match phase {
        PluginPhase::Zero => "zero",
        PluginPhase::RequestFilter => "request_filter",
        PluginPhase::ResponseFilter => "response_filter",
        PluginPhase::ResponseBodyFilter => "response_body_filter",
        PluginPhase::Logging => "logging",
    }
}

#[allow(dead_code)]
fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn map_messaging_error(plugin: &MessagingPlugin, err: MessagingError) -> NylonError {
    NylonError::RuntimeError(format!(
        "Messaging plugin '{}' error: {}",
        plugin.plugin_name(),
        err
    ))
}

#[allow(dead_code)]
fn map_protocol_error(plugin: &MessagingPlugin, err: nylon_messaging::ProtocolError) -> NylonError {
    NylonError::RuntimeError(format!(
        "Messaging plugin '{}' protocol error: {}",
        plugin.plugin_name(),
        err
    ))
}

pub fn register_plugin(plugin: &PluginItem) -> Result<(), NylonError> {
    let messaging_name = plugin
        .messaging
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            NylonError::ConfigError(format!(
                "Messaging plugin {} must reference a messaging config",
                plugin.name
            ))
        })?;

    let configs =
        nylon_store::get::<DashMap<String, MessagingConfig>>(KEY_MESSAGING_CONFIG).ok_or_else(
            || {
                NylonError::ConfigError(
                    "Messaging configuration store not initialized. Define `messaging:` in the config before plugins.".to_string(),
                )
            },
        )?;

    let Some(config_entry) = configs.get(messaging_name) else {
        return Err(NylonError::ConfigError(format!(
            "Messaging config '{}' referenced by plugin {} not found",
            messaging_name, plugin.name
        )));
    };

    let messaging_config = config_entry.clone();

    let messaging_plugin = Arc::new(MessagingPlugin::new(plugin, &messaging_config)?);
    let queue_group = messaging_plugin.queue_group().to_string();

    let plugins =
        match nylon_store::get::<DashMap<String, Arc<MessagingPlugin>>>(KEY_MESSAGING_PLUGINS) {
            Some(map) => map,
            None => {
                let map = DashMap::new();
                nylon_store::insert(KEY_MESSAGING_PLUGINS, map.clone());
                map
            }
        };

    plugins.insert(plugin.name.clone(), messaging_plugin);
    nylon_store::insert(KEY_MESSAGING_PLUGINS, plugins);

    debug!(
        plugin = %plugin.name,
        config = %messaging_name,
        queue = %queue_group,
        "Registered messaging plugin",
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_session<T>(
    proxy: &T,
    plugin_name: &str,
    phase: PluginPhase,
    entry: &str,
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<serde_json::Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    response_body: &Option<Bytes>,
    plugin: Arc<MessagingPlugin>,
) -> Result<crate::types::PluginResult, NylonError>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
    let key = format!("{}-{}", plugin_name, entry);
    let mut session_id = {
        let map = ctx
            .session_ids
            .read()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
        *map.get(&key).unwrap_or(&0)
    };

    if session_id == 0 {
        session_id = next_session_id();
        ctx.session_ids
            .write()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
            .insert(key.clone(), session_id);
    }

    let client = plugin
        .client()
        .await
        .map_err(|err| map_messaging_error(&plugin, err))?;

    // Create transport and handler with tracing
    let request_id = new_request_id();
    let trace_id = ctx
        .params
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().and_then(|map| map.get("x-trace-id").cloned()));
    
    let trace = TraceMeta {
        request_id: Some(request_id.to_string()),
        trace_id,
        span_id: Some(format!("{}:{}", plugin.plugin_name(), session_id)),
    };
    
    let request_subject = format!(
        "nylon.plugin.{}.{}",
        plugin.plugin_name(),
        phase_subject_fragment(phase.clone())
    );
    
    let reply_subject = format!(
        "nylon.plugin.{}.reply.{}",
        plugin.plugin_name(),
        session_id
    );
    
    let transport = MessagingTransport::new(
        trace,
        client.clone(),
        request_subject.clone(),
        session_id,
        plugin.plugin_name().to_string(),
    );
    
    // Setup reply subscription
    let setup_result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            transport.setup_reply_subscription(reply_subject.clone()).await
        })
    });
    
    if let Err(e) = setup_result {
        return Err(NylonError::RuntimeError(format!(
            "Failed to setup reply subscription: {}",
            e
        )));
    }
    
    let mut handler = TransportSessionHandler::new(transport);

    // Send start phase event
    let phase_code = phase.clone().to_u8();
    handler
        .start_phase(phase_code)
        .map_err(|e| NylonError::RuntimeError(format!("Failed to start phase: {}", e)))?;
    
    // Flush initial event to NATS
    let flush_result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            handler.transport_mut().flush_events().await
        })
    });
    
    if let Err(e) = flush_result {
        return Err(NylonError::RuntimeError(format!(
            "Failed to flush events: {}",
            e
        )));
    }

    // Get phase policy for timeout and retry
    let phase_policy = plugin
        .per_phase()
        .get(&MessagingPhase::RequestFilter)
        .cloned();
    
    let timeout_ms = phase_policy
        .as_ref()
        .and_then(|p| p.timeout.map(|d| d.as_millis() as u64))
        .unwrap_or(5000);
    
    let on_error = phase_policy
        .as_ref()
        .map(|p| p.on_error)
        .unwrap_or(MessagingOnError::Retry);
    
    // Process loop with retry support
    let mut attempts = 0;
    let max_attempts = phase_policy
        .as_ref()
        .map(|p| p.retry.max_attempts)
        .unwrap_or(1);
    
    loop {
        attempts += 1;
        
        match handler.process_loop(timeout_ms) {
            Ok(crate::transport_handler::PluginLoopResult::Invoke(inv)) => {
                // Received method invoke from plugin
                debug!(
                    plugin = %plugin.plugin_name(),
                    method = inv.method,
                    data_len = inv.data.len(),
                    "received invoke in messaging transport"
                );
                
                // Process method invoke
                match crate::messaging_methods::process_messaging_method(
                    proxy,
                    inv,
                    ctx,
                    session,
                    payload,
                    payload_ast,
                    response_body,
                )
                .await
                {
                    Ok(Some(result)) => return Ok(result),
                    Ok(None) => {
                        // Method processed, continue loop to wait for next invoke
                        continue;
                    }
                    Err(e) => {
                        // Handle error based on policy
                        match on_error {
                            MessagingOnError::Continue => {
                                debug!(
                                    plugin = %plugin.plugin_name(),
                                    error = %e,
                                    "error processing method, continuing"
                                );
                                continue;
                            }
                            MessagingOnError::End => {
                                return Err(e);
                            }
                            MessagingOnError::Retry => {
                                if attempts >= max_attempts {
                                    return Err(e);
                                }
                                debug!(
                                    plugin = %plugin.plugin_name(),
                                    error = %e,
                                    attempt = attempts,
                                    max_attempts,
                                    "retrying after error"
                                );
                                // Reset handler for retry
                                handler
                                    .start_phase(phase_code)
                                    .map_err(|e| NylonError::RuntimeError(format!("Failed to start phase on retry: {}", e)))?;
                                
                                let flush_result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current().block_on(async {
                                        handler.transport_mut().flush_events().await
                                    })
                                });
                                
                                if let Err(e) = flush_result {
                                    return Err(NylonError::RuntimeError(format!(
                                        "Failed to flush events on retry: {}",
                                        e
                                    )));
                                }
                                
                                continue;
                            }
                        }
                    }
                }
            }
            Ok(crate::transport_handler::PluginLoopResult::Timeout) => {
                // Timeout reached
                if attempts >= max_attempts {
                    debug!(
                        plugin = %plugin.plugin_name(),
                        "messaging transport timeout after {} attempts",
                        attempts
                    );
                    return Ok(crate::types::PluginResult::default());
                }
                
                // Retry on timeout if policy allows
                match on_error {
                    MessagingOnError::Retry => {
                        debug!(
                            plugin = %plugin.plugin_name(),
                            attempt = attempts,
                            max_attempts,
                            "retrying after timeout"
                        );
                        continue;
                    }
                    _ => return Ok(crate::types::PluginResult::default()),
                }
            }
            Err(e) => {
                // Transport error - retry or fail based on policy
                if attempts >= max_attempts || on_error != MessagingOnError::Retry {
                    return Err(NylonError::RuntimeError(format!(
                        "Transport error after {} attempts: {}",
                        attempts, e
                    )));
                }
                
                debug!(
                    plugin = %plugin.plugin_name(),
                    error = %e,
                    attempt = attempts,
                    max_attempts,
                    "retrying after transport error"
                );
                continue;
            }
        }
    }
}
