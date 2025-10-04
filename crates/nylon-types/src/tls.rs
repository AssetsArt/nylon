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
}
