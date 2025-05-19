use fnv::FnvHasher;
use nylon_error::NylonError;
use nylon_types::services::{Algorithm, ServiceItem, ServiceType};
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
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use crate::{KEY_SERVICES, get, insert};

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

pub async fn store_services(services: Vec<ServiceItem>) -> Result<(), NylonError> {
    let services = services
        .iter()
        .filter(|s| s.service_type == ServiceType::Http)
        .collect::<Vec<&ServiceItem>>();

    let mut store_services = HashMap::new();
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
            if backend
                .ext
                .insert::<HttpPeer>(HttpPeer::new(endpoint, false, String::new()))
                .is_some()
            {
                return Err(NylonError::ConfigError(
                    "Unable to insert HttpPeer".to_string(),
                ));
            }
            backends.insert(backend);
        }
        let disco = discovery::Static::new(backends);
        let backend_type = match service.algorithm {
            Some(Algorithm::RoundRobin) => {
                let upstreams =
                    LoadBalancer::<Weighted<RoundRobin>>::from_backends(Backends::new(disco));
                match upstreams.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::RoundRobin(Arc::new(upstreams))
            }
            Some(Algorithm::Weighted) => {
                let backend =
                    LoadBalancer::<Weighted<fnv::FnvHasher>>::from_backends(Backends::new(disco));
                match backend.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::Weighted(Arc::new(backend))
            }
            Some(Algorithm::Consistent) => {
                let backend = LoadBalancer::<KetamaHashing>::from_backends(Backends::new(disco));
                match backend.update().await {
                    Ok(_) => {}
                    Err(e) => {
                        return Err(NylonError::PingoraError(format!("{}", e)));
                    }
                }
                BackendType::Consistent(Arc::new(backend))
            }
            Some(Algorithm::Random) => {
                let upstreams =
                    LoadBalancer::<Weighted<Random>>::from_backends(Backends::new(disco));
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
        store_services.insert(
            service.name.clone(),
            HttpService {
                name: service.name.clone(),
                backend_type,
            },
        );
    }
    insert(KEY_SERVICES, store_services);
    Ok(())
}

pub async fn get_backend(service_name: &str) -> Result<HttpService, NylonError> {
    let Some(services) = get::<HashMap<String, HttpService>>(KEY_SERVICES) else {
        return Err(NylonError::ConfigError(format!(
            "Services not found: {}",
            service_name
        )));
    };
    match services.get(service_name) {
        Some(service) => Ok(service.clone()),
        None => Err(NylonError::ConfigError(format!(
            "Service not found: {}",
            service_name
        ))),
    }
}
