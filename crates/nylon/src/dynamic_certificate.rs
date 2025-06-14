use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_store::{routes, tls};
use openssl::{
    pkey::PKey,
    ssl::{NameType, SslRef},
    x509::X509,
};
use pingora::{
    listeners::{TlsAccept, tls::TlsSettings},
    tls::ext,
};
use tracing::error;

#[derive(Default)]
pub struct DynamicCertificate;

impl DynamicCertificate {
    pub fn new() -> Self {
        Self
    }
}

pub fn new_tls_settings() -> Result<TlsSettings, NylonError> {
    let mut tls = TlsSettings::with_callbacks(Box::new(DynamicCertificate::new()))
        .map_err(|e| NylonError::PingoraError(e.to_string()))?;
    tls.enable_h2();
    Ok(tls)
}

#[async_trait]
impl TlsAccept for DynamicCertificate {
    async fn certificate_callback(&self, ssl: &mut SslRef) {
        let server_name = ssl.servername(NameType::HOST_NAME);

        let server_name = match server_name {
            Some(s) => s,
            None => {
                error!("Unable to get server name");
                "localhost"
            }
        };

        // Enabled tls route?
        if let Err(e) = routes::get_tls_route(server_name) {
            error!("Unable to get TLS route: {}", e);
            return;
        }
        // debug!("server_name: {}", server_name);
        let tls_store = match tls::get_certs(server_name) {
            Ok(tls_store) => tls_store,
            Err(e) => {
                error!("Unable to get TLS store: {}", e);
                return;
            }
        };
        // debug!("tls_store: {:?}", tls_store);
        let cert = match X509::from_pem(&tls_store.cert) {
            Ok(cert) => cert,
            Err(e) => {
                error!("Failed to parse certificate: {}", e);
                return;
            }
        };
        let key = match PKey::private_key_from_pem(&tls_store.key) {
            Ok(key) => key,
            Err(e) => {
                error!("Failed to parse private key: {}", e);
                return;
            }
        };

        if let Err(e) = ext::ssl_use_certificate(ssl, &cert) {
            error!("Failed to use certificate: {}", e);
        }

        if let Err(e) = ext::ssl_use_private_key(ssl, &key) {
            error!("Failed to use private key: {}", e);
        }

        for chain in &tls_store.chain {
            let chain = match X509::from_pem(chain) {
                Ok(chain) => chain,
                Err(e) => {
                    error!("Failed to parse chain certificate: {}", e);
                    return;
                }
            };
            if let Err(e) = ext::ssl_add_chain_cert(ssl, &chain) {
                error!("Failed to add chain certificate: {}", e);
            }
        }
    }
}
