use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum TlsKind {
    #[serde(rename = "custom")]
    Custom,
    #[serde(rename = "acme")]
    Acme,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    #[serde(rename = "type")]
    pub kind: TlsKind, // "custom" or "acme"
    pub key: Option<String>,
    pub cert: Option<String>,
    pub chain: Option<Vec<String>>,
    pub acme: Option<AcmeConfig>,
    pub domains: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AcmeConfig {
    pub provider: String,
    pub email: String,
    /// Path to ACME directory (will use runtime config if not specified)
    #[serde(skip)]
    pub acme_dir: Option<String>,
    /// Use staging endpoint if provider supports it (e.g. Let's Encrypt)
    pub staging: Option<bool>,
    /// Custom ACME directory URL (overrides provider/staging)
    pub directory_url: Option<String>,
    /// External Account Binding key identifier (for providers like ZeroSSL)
    pub eab_kid: Option<String>,
    /// External Account Binding HMAC key (base64/urlsafe as required by provider)
    pub eab_hmac_key: Option<String>,
}
