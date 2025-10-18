use crate as store;
use fnv::FnvHasher;
use lru::LruCache;
use nylon_error::NylonError;
use nylon_types::services::{Algorithm, HealthCheck, ServiceItem, ServiceType};
use once_cell::sync::Lazy;
use pingora::http::RequestHeader;
use pingora::lb::health_check::HttpHealthCheck;
use pingora::{
    lb::{
        Backend, Backends, Extensions, LoadBalancer, discovery,
        selection::{
            algorithms::{Random, RoundRobin},
            consistent::KetamaHashing,
            weighted::Weighted,
        },
    },
    prelude::HttpPeer,
    protocols::l4::socket::SocketAddr,
};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::Duration;
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

// LRU cache for backend service lookups - cache up to 500 services
static BACKEND_SERVICE_CACHE: Lazy<Mutex<LruCache<String, HttpService>>> =
    Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(500).unwrap())));

#[derive(Clone)]
pub enum BackendType {
    RoundRobin(Arc<LoadBalancer<Weighted<RoundRobin>>>),
    Weighted(Arc<LoadBalancer<Weighted<FnvHasher>>>),
    Consistent(Arc<LoadBalancer<KetamaHashing>>),
    Random(Arc<LoadBalancer<Weighted<Random>>>),
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::RoundRobin(v) => {
                write!(f, "RoundRobin({:#?})", v.backends().get_backend())
            }
            BackendType::Weighted(v) => write!(f, "Weighted({:#?})", v.backends().get_backend()),
            BackendType::Consistent(v) => {
                write!(f, "Consistent({:#?})", v.backends().get_backend())
            }
            BackendType::Random(v) => {
                write!(f, "Random({:#?})", v.backends().get_backend())
            }
        }
    }
}

impl std::fmt::Debug for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, Clone)]
pub struct HttpService {
    pub name: String,
    pub backend_type: BackendType,
}

pub async fn store(services: &Vec<&ServiceItem>) -> Result<(), NylonError> {
    let services = services
        .iter()
        .filter(|s| s.service_type == ServiceType::Http);

    let mut store_backends = HashMap::new();
    for service in services {
        let mut backends: BTreeSet<Backend> = BTreeSet::new();
        for e in service.endpoints.iter().flatten() {
            let endpoint = format!("{}:{}", e.ip, e.port);
            let addr: SocketAddr = match endpoint.parse() {
                Ok(val) => val,
                Err(e) => {
                    return Err(NylonError::ConfigError(format!(
                        "Unable to parse address: {}",
                        e
                    )));
                }
            };
            let mut backend = Backend {
                addr,
                weight: e.weight.unwrap_or(1) as usize,
                ext: Extensions::new(),
            };
            backend
                .ext
                .insert::<HttpPeer>(HttpPeer::new(endpoint, false, String::new()));
            if let Some(health_check) = &service.health_check {
                backend.ext.insert::<HealthCheck>(health_check.clone());
            }
            backends.insert(backend);
        }
        let disco = discovery::Static::new(backends);
        // derive a host header for health checks (fallback ip if missing)
        let host_for_hc = service
            .endpoints
            .as_ref()
            .and_then(|v| v.first())
            .map(|e| e.ip.clone())
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let backend_type = match service.algorithm {
            Some(Algorithm::RoundRobin) => {
                let mut upstreams =
                    LoadBalancer::<Weighted<RoundRobin>>::from_backends(Backends::new(disco));
                // Configure health check if provided
                if let Some(hc) = &service.health_check
                    && hc.enabled
                {
                    let timeout_secs = parse_seconds(&hc.timeout).unwrap_or(1);
                    let mut check = HttpHealthCheck::new(&host_for_hc, false);
                    check.consecutive_success = hc.healthy_threshold as usize;
                    check.consecutive_failure = hc.unhealthy_threshold as usize;
                    check.peer_template.options.connection_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    check.peer_template.options.read_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    // override request path and host header
                    let mut req = RequestHeader::build("GET", hc.path.as_bytes(), None).unwrap();
                    let _ = req.append_header("Host", &host_for_hc);
                    check.req = req;
                    upstreams.set_health_check(Box::new(check));
                    upstreams.parallel_health_check = true;
                    upstreams.health_check_frequency = Some(Duration::from_secs(
                        parse_seconds(&hc.interval).unwrap_or(5),
                    ));
                }
                match upstreams.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::RoundRobin(Arc::new(upstreams))
            }
            Some(Algorithm::Weighted) => {
                let mut backend =
                    LoadBalancer::<Weighted<fnv::FnvHasher>>::from_backends(Backends::new(disco));
                if let Some(hc) = &service.health_check
                    && hc.enabled
                {
                    let timeout_secs = parse_seconds(&hc.timeout).unwrap_or(1);
                    let mut check = HttpHealthCheck::new(&host_for_hc, false);
                    check.consecutive_success = hc.healthy_threshold as usize;
                    check.consecutive_failure = hc.unhealthy_threshold as usize;
                    check.peer_template.options.connection_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    check.peer_template.options.read_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    let mut req = RequestHeader::build("GET", hc.path.as_bytes(), None).unwrap();
                    let _ = req.append_header("Host", &host_for_hc);
                    check.req = req;
                    backend.set_health_check(Box::new(check));
                    backend.parallel_health_check = true;
                    backend.health_check_frequency = Some(Duration::from_secs(
                        parse_seconds(&hc.interval).unwrap_or(5),
                    ));
                }
                match backend.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::Weighted(Arc::new(backend))
            }
            Some(Algorithm::Consistent) => {
                let mut backend =
                    LoadBalancer::<KetamaHashing>::from_backends(Backends::new(disco));
                if let Some(hc) = &service.health_check
                    && hc.enabled
                {
                    let timeout_secs = parse_seconds(&hc.timeout).unwrap_or(1);
                    let mut check = HttpHealthCheck::new(&host_for_hc, false);
                    check.consecutive_success = hc.healthy_threshold as usize;
                    check.consecutive_failure = hc.unhealthy_threshold as usize;
                    check.peer_template.options.connection_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    check.peer_template.options.read_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    let mut req = RequestHeader::build("GET", hc.path.as_bytes(), None).unwrap();
                    let _ = req.append_header("Host", &host_for_hc);
                    check.req = req;
                    backend.set_health_check(Box::new(check));
                    backend.parallel_health_check = true;
                    backend.health_check_frequency = Some(Duration::from_secs(
                        parse_seconds(&hc.interval).unwrap_or(5),
                    ));
                }
                match backend.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::Consistent(Arc::new(backend))
            }
            Some(Algorithm::Random) => {
                let mut upstreams =
                    LoadBalancer::<Weighted<Random>>::from_backends(Backends::new(disco));
                if let Some(hc) = &service.health_check
                    && hc.enabled
                {
                    let timeout_secs = parse_seconds(&hc.timeout).unwrap_or(1);
                    let mut check = HttpHealthCheck::new(&host_for_hc, false);
                    check.consecutive_success = hc.healthy_threshold as usize;
                    check.consecutive_failure = hc.unhealthy_threshold as usize;
                    check.peer_template.options.connection_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    check.peer_template.options.read_timeout =
                        Some(Duration::from_secs(timeout_secs));
                    let mut req = RequestHeader::build("GET", hc.path.as_bytes(), None).unwrap();
                    let _ = req.append_header("Host", &host_for_hc);
                    check.req = req;
                    upstreams.set_health_check(Box::new(check));
                    upstreams.parallel_health_check = true;
                    upstreams.health_check_frequency = Some(Duration::from_secs(
                        parse_seconds(&hc.interval).unwrap_or(5),
                    ));
                }
                match upstreams.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::Random(Arc::new(upstreams))
            }
            _ => {
                return Err(NylonError::ConfigError(format!(
                    "Unknown algorithm: {:?}",
                    service.algorithm
                )));
            }
        };
        store_backends.insert(
            service.name.clone(),
            HttpService {
                name: service.name.clone(),
                backend_type,
            },
        );
    }
    store::insert(store::KEY_LB_BACKENDS, store_backends);

    // Clear backend service cache when backends are reloaded
    clear_backend_service_cache();

    Ok(())
}

pub async fn get(service_name: &str) -> Result<HttpService, NylonError> {
    // Check cache first
    if let Ok(mut cache) = BACKEND_SERVICE_CACHE.lock()
        && let Some(cached) = cache.get(service_name)
    {
        tracing::debug!("Backend service cache hit: {}", service_name);
        return Ok(cached.clone());
    }

    tracing::debug!("Backend service cache miss: {}", service_name);

    // Cache miss - lookup from store
    let Some(services) = store::get::<HashMap<String, HttpService>>(store::KEY_LB_BACKENDS) else {
        return Err(NylonError::ConfigError(format!(
            "Services not found: {}",
            service_name
        )));
    };
    let service = services
        .get(service_name)
        .ok_or_else(|| NylonError::ConfigError(format!("Service not found: {}", service_name)))?
        .clone();

    // Store in cache
    if let Ok(mut cache) = BACKEND_SERVICE_CACHE.lock() {
        cache.put(service_name.to_string(), service.clone());
    }

    Ok(service)
}

/// Clear backend service cache - useful when services are reloaded
pub fn clear_backend_service_cache() {
    if let Ok(mut cache) = BACKEND_SERVICE_CACHE.lock() {
        cache.clear();
        tracing::info!("Backend service cache cleared");
    }
}

/// Get backend service cache statistics
pub fn get_backend_service_cache_stats() -> (usize, usize) {
    if let Ok(cache) = BACKEND_SERVICE_CACHE.lock() {
        (cache.len(), cache.cap().get())
    } else {
        (0, 0)
    }
}

/// Parse a duration string like "5s" into seconds
fn parse_seconds(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_suffix('s') {
        return stripped.parse::<u64>().ok();
    }
    trimmed.parse::<u64>().ok()
}

/// Run health checks for all stored HTTP services
pub async fn run_health_checks_for_all() {
    let Some(services) = store::get::<HashMap<String, HttpService>>(store::KEY_LB_BACKENDS) else {
        return;
    };
    for (_name, svc) in services.into_iter() {
        match svc.backend_type {
            BackendType::RoundRobin(lb) => {
                lb.backends().run_health_check(true).await;
            }
            BackendType::Weighted(lb) => {
                lb.backends().run_health_check(true).await;
            }
            BackendType::Consistent(lb) => {
                lb.backends().run_health_check(true).await;
            }
            BackendType::Random(lb) => {
                lb.backends().run_health_check(true).await;
            }
        }
    }
}
