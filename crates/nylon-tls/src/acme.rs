use instant_acme::{Account, AccountCredentials, Directory, DirectoryUrl};
use nylon_error::NylonError;
use nylon_types::tls::TlsConfig;
use std::path::{Path, PathBuf};
use std::fs;

/// Fetches or renews certificates for an ACME TLS config.
///
/// This is a best-effort implementation using the `instant-acme` crate.
/// The challenge handling is not fully implemented yet.
pub async fn ensure_certs(config: &TlsConfig, acme_dir: &Path) -> Result<(), NylonError> {
    let Some(acme) = &config.acme else { return Ok(()); };
    let provider = match acme.provider.as_str() {
        "letsencrypt" => DirectoryUrl::LetsEncrypt,
        "buypass" => DirectoryUrl::BuyPass,
        _ => DirectoryUrl::LetsEncrypt,
    };
    fs::create_dir_all(acme_dir).map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
    let creds_path = acme_dir.join("account.json");
    let directory = Directory::from_url(provider)
        .await
        .map_err(|e| NylonError::AcmeHttpClientError(e.to_string()))?;
    let account = if creds_path.exists() {
        let creds = fs::read_to_string(&creds_path)
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        let creds: AccountCredentials = serde_json::from_str(&creds)
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        Account::from_credentials(directory, creds)
            .await
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?
    } else {
        let acc = Account::create(directory, &acme.email)
            .await
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        let creds = acc.credentials();
        fs::write(&creds_path, serde_json::to_string(&creds).unwrap())
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        acc
    };

    // Request certificate order for the configured domains
    let mut order = account
        .new_order(config.domains.clone())
        .await
        .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;

    // TODO: Handle HTTP-01/ALPN challenges so ACME servers can verify ownership.
    // For now we simply create the order and wait for it to be ready.
    order.wait_ready().await.map_err(|e| NylonError::AcmeClientError(e.to_string()))?;

    let pkey_pri = order.generate_p256_key();
    let cert = order
        .finalize_pkey(pkey_pri)
        .await
        .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
    let cert = cert.download_and_save().await.map_err(|e| NylonError::AcmeClientError(e.to_string()))?;

    for domain in &config.domains {
        let domain_dir = acme_dir.join(domain);
        fs::create_dir_all(&domain_dir).map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        fs::write(domain_dir.join("cert.pem"), cert.certificate())
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
        fs::write(domain_dir.join("key.pem"), cert.private_key())
            .map_err(|e| NylonError::AcmeClientError(e.to_string()))?;
    }

    Ok(())
}
