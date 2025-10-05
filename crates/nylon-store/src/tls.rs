use crate::{KEY_ACME_CERTS, KEY_TLS, get, insert};
use nylon_error::NylonError;
use nylon_tls::CertificateInfo;
use nylon_types::tls::{TlsConfig, TlsKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TlsStore {
    pub cert: Vec<u8>,
    pub key: Vec<u8>,
    pub chain: Vec<Vec<u8>>,
}

pub fn store(tls: Vec<&TlsConfig>, acme_dir: Option<String>) -> Result<(), NylonError> {
    let mut tls_store = HashMap::new();
    let mut acme_configs = HashMap::new();

    for t in tls {
        match t.kind {
            TlsKind::Custom => {
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
                        let cert = std::fs::read(path)
                            .map_err(|e| NylonError::ConfigError(e.to_string()))?;
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
            }
            TlsKind::Acme => {
                // เก็บ ACME config สำหรับแต่ละ domain
                if let Some(mut acme_config) = t.acme.clone() {
                    // ตั้งค่า acme_dir จากที่ส่งมา
                    if acme_config.acme_dir.is_none() {
                        acme_config.acme_dir = acme_dir.clone();
                    }
                    for domain in &t.domains {
                        acme_configs.insert(domain.clone(), acme_config.clone());
                    }
                }
            }
        }
    }

    insert::<HashMap<String, TlsStore>>(KEY_TLS, tls_store);
    crate::insert(crate::KEY_ACME_CONFIG, acme_configs);

    // Initialize ACME certificates store only if it doesn't exist
    // Don't overwrite existing certificates on reload
    if get::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS).is_none() {
        insert::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS, HashMap::new());
    }

    Ok(())
}

pub fn get_certs(domain: &str) -> Result<TlsStore, NylonError> {
    // ลองหาจาก ACME certificates ก่อน
    if let Some(acme_certs) = get::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS)
        && let Some(cert_info) = acme_certs.get(domain)
    {
        return Ok(TlsStore {
            cert: cert_info.cert.clone(),
            key: cert_info.key.clone(),
            chain: cert_info.chain.clone(),
        });
    }

    // ถ้าไม่มีใน ACME ให้หาจาก custom certificates
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

/// เก็บ ACME certificate
pub fn store_acme_cert(cert_info: CertificateInfo) -> Result<(), NylonError> {
    let mut acme_certs =
        get::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS).unwrap_or_default();

    acme_certs.insert(cert_info.domain.clone(), cert_info);
    insert::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS, acme_certs);

    Ok(())
}

/// ดึงรายการ certificates ทั้งหมดที่ต้องตรวจสอบการ renew
pub fn get_all_certificates() -> Vec<CertificateInfo> {
    let acme_certs = get::<HashMap<String, CertificateInfo>>(KEY_ACME_CERTS).unwrap_or_default();

    acme_certs.values().cloned().collect()
}
