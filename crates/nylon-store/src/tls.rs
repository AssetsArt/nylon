use crate::{KEY_TLS, get, insert};
use nylon_error::NylonError;
use nylon_types::tls::{TlsConfig, TlsKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TlsStore {
    pub cert: Vec<u8>,
    pub key: Vec<u8>,
    pub chain: Vec<Vec<u8>>,
}

use std::path::Path;

pub fn store(tls: Vec<&TlsConfig>, acme_dir: &Path) -> Result<(), NylonError> {
    let mut tls_store = HashMap::new();
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
    for t in tls {
        if t.kind == TlsKind::Custom {
            // store custom tls
            let Some(path_cert) = &t.cert else {
                return Err(NylonError::ConfigError(
                    "Custom TLS certificate path is required".to_string(),
                ));
            };
            let Some(path_key) = &t.key else {
                return Err(NylonError::ConfigError(
                    "Custom TLS key path is required".to_string(),
                ));
            };
            let cert =
                std::fs::read(path_cert).map_err(|e| NylonError::ConfigError(e.to_string()))?;
            let key =
                std::fs::read(path_key).map_err(|e| NylonError::ConfigError(e.to_string()))?;
            let mut chain = vec![];
            if let Some(chain_path) = &t.chain {
                for path in chain_path {
                    let cert =
                        std::fs::read(path).map_err(|e| NylonError::ConfigError(e.to_string()))?;
                    chain.push(cert);
                }
            }
            for domain in t.domains.clone() {
                tls_store.insert(
                    domain,
                    TlsStore {
                        cert: cert.clone(),
                        key: key.clone(),
                        chain: chain.clone(),
                    },
                );
            }
        } else if t.kind == TlsKind::Acme {
            rt.block_on(nylon_tls::acme::ensure_certs(t, acme_dir))?;
            for domain in &t.domains {
                let cert_path = acme_dir.join(domain).join("cert.pem");
                let key_path = acme_dir.join(domain).join("key.pem");
                let cert = std::fs::read(&cert_path)
                    .map_err(|e| NylonError::ConfigError(e.to_string()))?;
                let key = std::fs::read(&key_path)
                    .map_err(|e| NylonError::ConfigError(e.to_string()))?;
                tls_store.insert(
                    domain.clone(),
                    TlsStore {
                        cert,
                        key,
                        chain: vec![],
                    },
                );
            }
        }
    }
    insert::<HashMap<String, TlsStore>>(KEY_TLS, tls_store);
    Ok(())
}

pub fn get_certs(domain: &str) -> Result<TlsStore, NylonError> {
    let tls_store = get::<HashMap<String, TlsStore>>(KEY_TLS).ok_or(NylonError::ConfigError(
        format!("TLS domain {} not found", domain),
    ))?;
    let tls_store = tls_store
        .get(domain)
        .ok_or(NylonError::ConfigError(format!(
            "TLS domain {} not found",
            domain
        )))?;
    Ok(tls_store.clone())
}
