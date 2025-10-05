use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt,
    NewAccount, NewOrder, OrderStatus, RetryPolicy,
};
use nylon_error::NylonError;
use nylon_types::tls::AcmeConfig;
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Rate limiting state สำหรับป้องกันการทำงานเร็วเกินไป
struct RateLimiter {
    last_request: Option<Instant>,
    backoff_duration: Duration,
    min_interval: Duration,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: None,
            backoff_duration: Duration::from_millis(1000),
            min_interval: Duration::from_secs(5),
        }
    }

    /// รอ delay ตาม rate limit
    async fn wait_if_needed(&mut self) {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            let required_wait = self.backoff_duration.max(self.min_interval);

            if elapsed < required_wait {
                let wait_time = required_wait - elapsed;
                info!("Rate limiting: waiting {:?}", wait_time);
                tokio::time::sleep(wait_time).await;
            }
        }
        self.last_request = Some(Instant::now());
    }

    /// เพิ่ม backoff หลังจากเกิด error
    fn increase_backoff(&mut self) {
        self.backoff_duration = (self.backoff_duration * 2).min(Duration::from_secs(300));
        warn!("Increased backoff to {:?}", self.backoff_duration);
    }

    /// Reset backoff หลังจากสำเร็จ
    fn reset_backoff(&mut self) {
        self.backoff_duration = Duration::from_millis(1000);
    }
}

/// ACME Client สำหรับจัดการ certificate ด้วย Let's Encrypt
pub struct AcmeClient {
    account: Account,
    acme_dir: String,
    rate_limiter: RateLimiter,
}

impl AcmeClient {
    /// สร้าง ACME client ใหม่
    pub async fn new(config: &AcmeConfig) -> Result<Self, NylonError> {
        info!("Creating ACME client for email: {}", config.email);

        let acme_dir = config
            .acme_dir
            .clone()
            .unwrap_or_else(|| ".acme".to_string());

        // Ensure directory exists and canonicalize when possible
        if let Err(e) = std::fs::create_dir_all(&acme_dir) {
            return Err(NylonError::ConfigError(format!(
                "Failed to create ACME dir: {}",
                e
            )));
        }
        let acme_dir = match std::fs::canonicalize(&acme_dir) {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => acme_dir,
        };

        info!("Using ACME directory: {}", acme_dir);

        // สร้าง account ใหม่หรือใช้ account ที่มีอยู่
        let account = match Self::load_account_credentials(&acme_dir) {
            Ok(credentials) => {
                info!("Using existing ACME account");
                Account::builder()
                    .map_err(|e| {
                        NylonError::ConfigError(format!("Failed to build account: {}", e))
                    })?
                    .from_credentials(credentials)
                    .await
                    .map_err(|e| {
                        NylonError::ConfigError(format!("Failed to load account: {}", e))
                    })?
            }
            Err(_) => {
                info!("Creating new ACME account");
                let (account, credentials) = Self::create_new_account(config).await?;
                Self::save_account_credentials(&credentials, &acme_dir)?;
                account
            }
        };

        Ok(Self {
            account,
            acme_dir,
            rate_limiter: RateLimiter::new(),
        })
    }

    /// สร้าง account ใหม่
    async fn create_new_account(
        config: &AcmeConfig,
    ) -> Result<(Account, AccountCredentials), NylonError> {
        let new_account = NewAccount {
            contact: &[&format!("mailto:{}", config.email)],
            terms_of_service_agreed: true,
            only_return_existing: false,
        };

        // Resolve directory URL
        let directory_url = if let Some(url) = &config.directory_url {
            url.clone()
        } else {
            let provider = config.provider.to_lowercase();
            if provider == "letsencrypt" {
                if config.staging.unwrap_or(false) {
                    LetsEncrypt::Staging.url().to_owned()
                } else {
                    LetsEncrypt::Production.url().to_owned()
                }
            } else {
                warn!(
                    "Unknown ACME provider '{}', defaulting to Let's Encrypt Production",
                    provider
                );
                LetsEncrypt::Production.url().to_owned()
            }
        };

        // Prepare EAB (External Account Binding) if provided
        // Note: EAB is provider-specific (e.g., ZeroSSL, BuyPass)
        // For now, we pass None and let instant-acme handle it
        // TODO: Full EAB implementation would need proper key parsing
        let eab = match (&config.eab_kid, &config.eab_hmac_key) {
            (Some(_), Some(_)) => {
                info!("EAB credentials provided (kid and hmac_key)");
                warn!(
                    "Full EAB support is not yet implemented. Please use Let's Encrypt or configure manually."
                );
                None
            }
            (Some(_), None) | (None, Some(_)) => {
                warn!("Incomplete EAB credentials (need both kid and hmac_key), ignoring");
                None
            }
            (None, None) => None,
        };

        let (account, credentials) = Account::builder()
            .map_err(|e| NylonError::ConfigError(format!("Failed to build account: {}", e)))?
            .create(&new_account, directory_url, eab)
            .await
            .map_err(|e| {
                NylonError::ConfigError(format!("Failed to create ACME account: {}", e))
            })?;

        Ok((account, credentials))
    }

    /// โหลด account credentials จาก file
    fn load_account_credentials(acme_dir: &str) -> Result<AccountCredentials, NylonError> {
        let path = Self::credentials_path(acme_dir);
        let data = std::fs::read_to_string(&path).map_err(|e| {
            NylonError::ConfigError(format!("Failed to read credentials file: {}", e))
        })?;

        let credentials: AccountCredentials = serde_json::from_str(&data)
            .map_err(|e| NylonError::ConfigError(format!("Failed to parse credentials: {}", e)))?;

        Ok(credentials)
    }

    /// บันทึก account credentials ลง file
    fn save_account_credentials(
        credentials: &AccountCredentials,
        acme_dir: &str,
    ) -> Result<(), NylonError> {
        let path = Self::credentials_path(acme_dir);

        // สร้างโฟลเดอร์ถ้ายังไม่มี
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NylonError::ConfigError(format!("Failed to create credentials directory: {}", e))
            })?;
        }

        let data = serde_json::to_string_pretty(credentials).map_err(|e| {
            NylonError::ConfigError(format!("Failed to serialize credentials: {}", e))
        })?;

        std::fs::write(&path, data).map_err(|e| {
            NylonError::ConfigError(format!("Failed to write credentials file: {}", e))
        })?;

        info!("Saved ACME credentials to: {}", path.display());
        Ok(())
    }

    /// ได้ path สำหรับเก็บ credentials
    fn credentials_path(acme_dir: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/account.json", acme_dir))
    }

    /// ได้ path สำหรับเก็บ certificate
    fn cert_path(acme_dir: &str, domain: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/certs/{}/cert.pem", acme_dir, domain))
    }

    /// ได้ path สำหรับเก็บ private key
    fn key_path(acme_dir: &str, domain: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/certs/{}/key.pem", acme_dir, domain))
    }

    /// ได้ path สำหรับเก็บ full chain
    fn fullchain_path(acme_dir: &str, domain: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/certs/{}/fullchain.pem", acme_dir, domain))
    }

    /// ได้ path สำหรับเก็บ intermediates chain (ไม่รวม leaf)
    fn chain_path(acme_dir: &str, domain: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/certs/{}/chain.pem", acme_dir, domain))
    }

    /// ได้ path สำหรับเก็บ challenge token
    fn challenge_path(acme_dir: &str, domain: &str, token: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("{}/challenges/{}/{}", acme_dir, domain, token))
    }

    /// ออก certificate ใหม่สำหรับ domain
    pub async fn issue_certificate(
        &mut self,
        domain: &str,
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<Vec<u8>>), NylonError> {
        info!("Issuing certificate for domain: {}", domain);

        // Apply rate limiting
        self.rate_limiter.wait_if_needed().await;

        // Track challenge tokens for cleanup
        let mut challenge_tokens: Vec<String> = Vec::new();

        // Main certificate issuance logic wrapped in error handling
        let result: Result<(Vec<u8>, Vec<u8>, Vec<Vec<u8>>), NylonError> = async {
            // สร้าง order ใหม่
            let identifiers = vec![Identifier::Dns(domain.to_string())];
            let mut order = self
                .account
                .new_order(&NewOrder::new(&identifiers))
                .await
                .map_err(|e| NylonError::ConfigError(format!("Failed to create order: {}", e)))?;

            info!("Order created for domain: {}", domain);

            // ดึง authorizations
            let mut authorizations = order.authorizations();

            // ทำ HTTP-01 challenge
            while let Some(authz_result) = authorizations.next().await {
                let mut authz = authz_result.map_err(|e| {
                    NylonError::ConfigError(format!("Failed to get authorization: {}", e))
                })?;

                match authz.status {
                    AuthorizationStatus::Pending => {}
                    AuthorizationStatus::Valid => continue,
                    _ => {
                        return Err(NylonError::ConfigError(format!(
                            "Authorization status is {:?}",
                            authz.status
                        )));
                    }
                }

                // หา HTTP-01 challenge
                let mut challenge = authz.challenge(ChallengeType::Http01).ok_or_else(|| {
                    NylonError::ConfigError("HTTP-01 challenge not found".to_string())
                })?;

                let token = challenge.token.clone();
                let key_auth = challenge.key_authorization().as_str().to_string();

                info!(
                    "HTTP-01 Challenge for {}: token={}, path=/.well-known/acme-challenge/{}",
                    domain, token, token
                );

                // บันทึก challenge token เพื่อให้ web server ให้บริการ
                Self::save_challenge_token(&self.acme_dir, domain, &token, &key_auth)?;
                challenge_tokens.push(token.clone());

                // แจ้ง ACME server ว่าพร้อมสำหรับการตรวจสอบ
                challenge.set_ready().await.map_err(|e| {
                    NylonError::ConfigError(format!("Failed to set challenge ready: {}", e))
                })?;

                info!("Challenge set to ready, waiting for validation...");

                // รอการตรวจสอบโดย poll authorizations ใหม่
                // instant-acme จะ handle polling ให้เอง
            }

            // Poll order จนกว่า order จะ ready
            let status = order
                .poll_ready(&RetryPolicy::default())
                .await
                .map_err(|e| {
                    NylonError::ConfigError(format!("Failed to poll order ready: {}", e))
                })?;

            if status != OrderStatus::Ready {
                return Err(NylonError::ConfigError(format!(
                    "Order status is not ready: {:?}",
                    status
                )));
            }

            info!("Order is ready, finalizing certificate...");

            // Finalize order - instant-acme จะสร้าง private key ให้เอง
            let private_key_pem = order
                .finalize()
                .await
                .map_err(|e| NylonError::ConfigError(format!("Failed to finalize order: {}", e)))?;

            // Poll certificate
            let cert_chain = order
                .poll_certificate(&RetryPolicy::default())
                .await
                .map_err(|e| {
                    NylonError::ConfigError(format!("Failed to download certificate: {}", e))
                })?;

            info!("Certificate issued successfully for domain: {}", domain);

            // แยก certificate และ chain
            let (cert_pem, chain_pems) = Self::split_certificate_chain(&cert_chain)?;
            let private_key = private_key_pem.as_bytes().to_vec();

            // บันทึก certificate, chain และ key
            Self::save_certificate_bundle(
                &self.acme_dir,
                domain,
                &cert_pem,
                &chain_pems,
                &private_key,
            )?;

            Ok((cert_pem, private_key, chain_pems))
        }
        .await;

        // Cleanup challenge tokens regardless of success or failure
        if !challenge_tokens.is_empty() {
            Self::cleanup_domain_challenges(&self.acme_dir, domain);
        }

        // Update rate limiter based on result
        match &result {
            Ok(_) => {
                self.rate_limiter.reset_backoff();
                info!(
                    "Certificate issuance completed successfully for: {}",
                    domain
                );
            }
            Err(e) => {
                self.rate_limiter.increase_backoff();
                error!("Failed to issue certificate for {}: {}", domain, e);
            }
        }

        result
    }

    /// แยก certificate chain
    fn split_certificate_chain(cert_chain: &str) -> Result<(Vec<u8>, Vec<Vec<u8>>), NylonError> {
        // Split by the complete PEM boundary to preserve structure
        let parts: Vec<&str> = cert_chain
            .split("-----BEGIN CERTIFICATE-----")
            .filter(|s| !s.trim().is_empty())
            .collect();

        if parts.is_empty() {
            return Err(NylonError::ConfigError(
                "No certificates found in chain".to_string(),
            ));
        }

        let mut result_certs = Vec::new();
        for part in parts {
            // Each part should contain the cert body + END marker
            // Reconstruct the complete PEM with proper boundaries
            let cert_pem = format!("-----BEGIN CERTIFICATE-----{}", part);
            // Ensure proper line ending after the END marker
            let cert_pem = if !cert_pem.ends_with('\n') {
                format!("{}\n", cert_pem)
            } else {
                cert_pem
            };
            result_certs.push(cert_pem.into_bytes());
        }

        // Certificate แรกคือ leaf certificate
        let cert = result_certs.remove(0);
        // ที่เหลือคือ intermediate certificates
        let chain = result_certs;

        Ok((cert, chain))
    }

    /// บันทึก certificate bundle (leaf + chain + fullchain) และ key ลง file
    fn save_certificate_bundle(
        acme_dir: &str,
        domain: &str,
        cert: &[u8],
        chain: &[Vec<u8>],
        key: &[u8],
    ) -> Result<(), NylonError> {
        let cert_path = Self::cert_path(acme_dir, domain);
        let key_path = Self::key_path(acme_dir, domain);
        let fullchain_path = Self::fullchain_path(acme_dir, domain);
        let chain_path = Self::chain_path(acme_dir, domain);

        // สร้างโฟลเดอร์
        if let Some(parent) = cert_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NylonError::ConfigError(format!("Failed to create cert directory: {}", e))
            })?;
        }

        // Prepare fullchain bytes
        let mut fullchain: Vec<u8> = Vec::new();
        fullchain.extend_from_slice(cert);
        for c in chain.iter() {
            fullchain.extend_from_slice(c);
        }

        // Write files with restrictive permissions
        #[cfg(unix)]
        let mut cert_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&cert_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write certificate: {}", e)))?;
        #[cfg(not(unix))]
        let mut cert_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&cert_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write certificate: {}", e)))?;
        use std::io::Write as _;
        cert_file
            .write_all(cert)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write certificate: {}", e)))?;

        #[cfg(unix)]
        let mut key_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&key_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write private key: {}", e)))?;
        #[cfg(not(unix))]
        let mut key_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&key_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write private key: {}", e)))?;
        key_file
            .write_all(key)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write private key: {}", e)))?;

        #[cfg(unix)]
        let mut fullchain_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&fullchain_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write fullchain: {}", e)))?;
        #[cfg(not(unix))]
        let mut fullchain_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&fullchain_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write fullchain: {}", e)))?;
        fullchain_file
            .write_all(&fullchain)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write fullchain: {}", e)))?;

        // Write chain (intermediates only)
        #[cfg(unix)]
        let mut chain_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&chain_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write chain: {}", e)))?;
        #[cfg(not(unix))]
        let mut chain_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&chain_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write chain: {}", e)))?;
        for c in chain.iter() {
            chain_file
                .write_all(c)
                .map_err(|e| NylonError::ConfigError(format!("Failed to write chain: {}", e)))?;
        }

        info!("Certificate saved to: {}", cert_path.display());
        info!("Full chain saved to: {}", fullchain_path.display());
        info!("Private key saved to: {}", key_path.display());

        Ok(())
    }

    /// โหลด certificate และ key จาก file
    pub fn load_certificate(
        acme_dir: &str,
        domain: &str,
    ) -> Result<(Vec<u8>, Vec<u8>), NylonError> {
        let cert_path = Self::cert_path(acme_dir, domain);
        let key_path = Self::key_path(acme_dir, domain);

        let cert = std::fs::read(&cert_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to read certificate: {}", e)))?;

        let key = std::fs::read(&key_path)
            .map_err(|e| NylonError::ConfigError(format!("Failed to read private key: {}", e)))?;

        Ok((cert, key))
    }

    /// โหลด certificate, key และ chain จาก file (ถ้ามี)
    pub fn load_certificate_with_chain(
        acme_dir: &str,
        domain: &str,
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<Vec<u8>>), NylonError> {
        let (cert, key) = Self::load_certificate(acme_dir, domain)?;
        let chain_path = Self::chain_path(acme_dir, domain);
        let chain = if chain_path.exists() {
            let data = std::fs::read(&chain_path)
                .map_err(|e| NylonError::ConfigError(format!("Failed to read chain: {}", e)))?;
            // Split concatenated PEMs by BEGIN marker to preserve structure
            let chain_str = String::from_utf8_lossy(&data);
            let parts: Vec<Vec<u8>> = chain_str
                .split("-----BEGIN CERTIFICATE-----")
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    let cert_pem = format!("-----BEGIN CERTIFICATE-----{}", s);
                    // Ensure proper line ending
                    if cert_pem.ends_with('\n') {
                        cert_pem.into_bytes()
                    } else {
                        format!("{}\n", cert_pem).into_bytes()
                    }
                })
                .collect();
            parts
        } else {
            Vec::new()
        };
        Ok((cert, key, chain))
    }

    /// บันทึก challenge token
    fn save_challenge_token(
        acme_dir: &str,
        domain: &str,
        token: &str,
        key_auth: &str,
    ) -> Result<(), NylonError> {
        let path = Self::challenge_path(acme_dir, domain, token);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NylonError::ConfigError(format!("Failed to create challenge directory: {}", e))
            })?;
        }

        // Write token with restrictive permissions
        #[cfg(unix)]
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| {
                NylonError::ConfigError(format!("Failed to write challenge token: {}", e))
            })?;
        #[cfg(not(unix))]
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| {
                NylonError::ConfigError(format!("Failed to write challenge token: {}", e))
            })?;
        use std::io::Write as _;
        f.write_all(key_auth.as_bytes()).map_err(|e| {
            NylonError::ConfigError(format!("Failed to write challenge token: {}", e))
        })?;

        info!("Challenge token saved to: {}", path.display());
        Ok(())
    }

    /// โหลด challenge token
    pub fn load_challenge_token(
        acme_dir: &str,
        domain: &str,
        token: &str,
    ) -> Result<String, NylonError> {
        let path = Self::challenge_path(acme_dir, domain, token);

        let key_auth = std::fs::read_to_string(&path).map_err(|e| {
            NylonError::ConfigError(format!("Failed to read challenge token: {}", e))
        })?;

        Ok(key_auth)
    }

    /// ลบ challenge tokens ทั้งหมดของ domain
    fn cleanup_domain_challenges(acme_dir: &str, domain: &str) {
        let challenge_dir = std::path::PathBuf::from(format!("{}/challenges/{}", acme_dir, domain));

        if challenge_dir.exists() {
            match std::fs::remove_dir_all(&challenge_dir) {
                Ok(_) => info!("Cleaned up challenge tokens for domain: {}", domain),
                Err(e) => warn!("Failed to cleanup challenge tokens for {}: {}", domain, e),
            }
        }
    }
}
