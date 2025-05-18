use async_trait::async_trait;
use nylon_error::NylonError;
use openssl::ssl::{NameType, SslRef};
use pingora::listeners::{TlsAccept, tls::TlsSettings};
use tracing::{debug, error};

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

        debug!("server_name: {}", server_name);

        todo!("dynamic certificate")
    }
}
