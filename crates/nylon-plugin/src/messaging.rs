use bytes::Bytes;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_messaging::{
    MessageHeaders, MessagingError, NatsClient, NatsClientOptions, PROTOCOL_VERSION,
    ResponseAction, RetryPolicy, decode_response, encode_request, new_request_id,
};
use nylon_store::{self, KEY_MESSAGING_CONFIG, KEY_MESSAGING_PLUGINS};
use nylon_types::context::NylonContext;
use nylon_types::plugins::{
    MessagingAuthConfig, MessagingConfig, MessagingOnError, MessagingPhase, MessagingPhaseConfig,
    MessagingTlsConfig, OverflowPolicy, PluginItem, PluginPhase, RetryPolicyConfig,
};
use nylon_types::template::Expr;
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
use tracing::{debug, warn};

#[derive(Clone, Debug)]
pub struct PhasePolicy {
    pub timeout: Option<Duration>,
    pub on_error: MessagingOnError,
    pub retry: RetryPolicy,
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
    let mut policies = HashMap::new();

    if let Some(configs) = per_phase {
        for (phase, cfg) in configs {
            let timeout = cfg.timeout_ms.map(Duration::from_millis);
            let on_error = cfg.on_error.unwrap_or(MessagingOnError::Retry);
            let retry = merge_retry_policy(base_retry, cfg.retry.as_ref());
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
    let _ = (proxy, session, payload, payload_ast, response_body);

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

    let request_id = new_request_id();

    let phase_code = phase.clone().to_u8();
    let request = nylon_messaging::PluginRequest {
        version: PROTOCOL_VERSION,
        request_id,
        session_id,
        phase: phase_code,
        method: 0,
        data: Vec::new(),
        timestamp: now_unix_millis(),
        headers: None,
    };

    let payload = encode_request(&request).map_err(|err| map_protocol_error(&plugin, err))?;

    let subject = format!(
        "nylon.plugin.{}.{}",
        plugin.plugin_name(),
        phase_subject_fragment(phase)
    );

    let response_bytes = client
        .request(&subject, &payload, None)
        .await
        .map_err(|err| map_messaging_error(&plugin, err))?;

    let response =
        decode_response(&response_bytes).map_err(|err| map_protocol_error(&plugin, err))?;

    if response.version != PROTOCOL_VERSION {
        warn!(
            plugin = %plugin.plugin_name(),
            expected = PROTOCOL_VERSION,
            actual = response.version,
            "Messaging protocol version mismatch"
        );
    }

    if response.request_id != request_id {
        warn!(
            plugin = %plugin.plugin_name(),
            expected = %request_id,
            actual = %response.request_id,
            "Messaging response request_id mismatch"
        );
    }

    match response.action {
        ResponseAction::Next => {
            if let Some(method) = response.method {
                debug!(
                    plugin = %plugin.plugin_name(),
                    method,
                    data_len = response.data.len(),
                    "received messaging method without executor (TODO)"
                );
            }
            Ok(crate::types::PluginResult::default())
        }
        ResponseAction::End => Ok(crate::types::PluginResult::new(true, false)),
        ResponseAction::Error => {
            Err(NylonError::RuntimeError(response.error.unwrap_or_else(
                || "messaging plugin returned error".to_string(),
            )))
        }
    }
}
