use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub enum TlsKind {
    #[serde(rename = "custom")]
    Custom,
    #[serde(rename = "acme")]
    Acme,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: TlsKind, // "custom" or "acme"
    pub key: Option<String>,
    pub cert: Option<String>,
    pub chain: Option<Vec<String>>,
    pub acme: Option<AcmeConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AcmeConfig {
    pub provider: String,
    pub email: String,
}
