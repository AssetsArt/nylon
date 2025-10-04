use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt,
    NewAccount, NewOrder, OrderStatus, RetryPolicy,
};
use nylon_error::NylonError;
use nylon_types::tls::AcmeConfig;
use tracing::info;

/// ACME Client สำหรับจัดการ certificate ด้วย Let's Encrypt
pub struct AcmeClient {
    account: Account,
    acme_dir: String,
}

impl AcmeClient {
    /// สร้าง ACME client ใหม่
    pub async fn new(config: &AcmeConfig) -> Result<Self, NylonError> {
        info!("Creating ACME client for email: {}", config.email);

        let acme_dir = config
            .acme_dir
            .clone()
            .unwrap_or_else(|| ".acme".to_string());

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
                let (account, credentials) = Self::create_new_account(&config.email).await?;
                Self::save_account_credentials(&credentials, &acme_dir)?;
                account
            }
        };

        Ok(Self { account, acme_dir })
    }

    /// สร้าง account ใหม่
    async fn create_new_account(email: &str) -> Result<(Account, AccountCredentials), NylonError> {
        let new_account = NewAccount {
            contact: &[&format!("mailto:{}", email)],
            terms_of_service_agreed: true,
            only_return_existing: false,
        };

        let (account, credentials) = Account::builder()
            .map_err(|e| NylonError::ConfigError(format!("Failed to build account: {}", e)))?
            .create(&new_account, LetsEncrypt::Production.url().to_owned(), None)
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
            info!("Key authorization: {}", key_auth);

            // บันทึก challenge token เพื่อให้ web server ให้บริการ
            Self::save_challenge_token(&self.acme_dir, domain, &token, &key_auth)?;

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
            .map_err(|e| NylonError::ConfigError(format!("Failed to poll order ready: {}", e)))?;

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

        // บันทึก certificate และ key
        Self::save_certificate(&self.acme_dir, domain, &cert_pem, &private_key)?;

        Ok((cert_pem, private_key, chain_pems))
    }

    /// แยก certificate chain
    fn split_certificate_chain(cert_chain: &str) -> Result<(Vec<u8>, Vec<Vec<u8>>), NylonError> {
        let certs: Vec<&str> = cert_chain
            .split("-----END CERTIFICATE-----")
            .filter(|s| !s.trim().is_empty())
            .collect();

        if certs.is_empty() {
            return Err(NylonError::ConfigError(
                "No certificates found in chain".to_string(),
            ));
        }

        let mut result_certs = Vec::new();
        for cert_str in certs {
            let cert_pem = format!("{}-----END CERTIFICATE-----\n", cert_str.trim());
            result_certs.push(cert_pem.into_bytes());
        }

        // Certificate แรกคือ leaf certificate
        let cert = result_certs.remove(0);
        // ที่เหลือคือ intermediate certificates
        let chain = result_certs;

        Ok((cert, chain))
    }

    /// บันทึก certificate และ key ลง file
    fn save_certificate(
        acme_dir: &str,
        domain: &str,
        cert: &[u8],
        key: &[u8],
    ) -> Result<(), NylonError> {
        let cert_path = Self::cert_path(acme_dir, domain);
        let key_path = Self::key_path(acme_dir, domain);

        // สร้างโฟลเดอร์
        if let Some(parent) = cert_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                NylonError::ConfigError(format!("Failed to create cert directory: {}", e))
            })?;
        }

        // บันทึก cert
        std::fs::write(&cert_path, cert)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write certificate: {}", e)))?;

        // บันทึก key
        std::fs::write(&key_path, key)
            .map_err(|e| NylonError::ConfigError(format!("Failed to write private key: {}", e)))?;

        info!("Certificate saved to: {}", cert_path.display());
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

        std::fs::write(&path, key_auth).map_err(|e| {
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
}
