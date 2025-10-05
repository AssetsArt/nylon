use chrono::{DateTime, NaiveDateTime, Utc};
use nylon_error::NylonError;
use openssl::asn1::Asn1TimeRef;
use openssl::x509::X509;
use serde::{Deserialize, Serialize};

/// ข้อมูล certificate พร้อมวันหมดอายุ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    pub domain: String,
    pub cert: Vec<u8>,
    pub key: Vec<u8>,
    pub chain: Vec<Vec<u8>>,
    pub expires_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}

impl CertificateInfo {
    /// สร้าง CertificateInfo จาก certificate และ key
    pub fn new(
        domain: String,
        cert: Vec<u8>,
        key: Vec<u8>,
        chain: Vec<Vec<u8>>,
    ) -> Result<Self, NylonError> {
        let x509 = X509::from_pem(&cert)
            .map_err(|e| NylonError::ConfigError(format!("Failed to parse certificate: {}", e)))?;

        let not_after = x509.not_after();
        let not_before = x509.not_before();

        // แปลง ASN1Time เป็น DateTime<Utc>
        let expires_at = asn1_time_to_datetime(not_after)?;
        let issued_at = asn1_time_to_datetime(not_before)?;

        Ok(Self {
            domain,
            cert,
            key,
            chain,
            expires_at,
            issued_at,
        })
    }

    /// ตรวจสอบว่า certificate ใกล้หมดอายุหรือไม่ (น้อยกว่า 30 วัน)
    pub fn needs_renewal(&self) -> bool {
        let now = Utc::now();
        let days_until_expiry = (self.expires_at - now).num_days();
        days_until_expiry < 30
    }

    /// ตรวจสอบว่า certificate หมดอายุแล้วหรือไม่
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// คำนวณจำนวนวันที่เหลือจนถึงวันหมดอายุ
    pub fn days_until_expiry(&self) -> i64 {
        (self.expires_at - Utc::now()).num_days()
    }
}

/// แปลง ASN1Time เป็น DateTime<Utc> โดย parse string format
fn asn1_time_to_datetime(asn1_time: &Asn1TimeRef) -> Result<DateTime<Utc>, NylonError> {
    // ASN1Time format examples:
    // "Oct  5 10:02:11 2025 GMT" (2 spaces for single digit day)
    // "Oct 15 10:02:11 2025 GMT" (1 space for double digit day)
    let time_str = asn1_time.to_string();
    
    // Try parsing with various formats
    // Format 1: "Oct  5 10:02:11 2025 GMT" (with GMT suffix)
    if let Ok(naive) = NaiveDateTime::parse_from_str(&time_str, "%b %e %H:%M:%S %Y GMT") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    
    // Format 2: "Oct  5 10:02:11 2025" (without GMT suffix)
    if let Ok(naive) = NaiveDateTime::parse_from_str(&time_str, "%b %e %H:%M:%S %Y") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    
    // Format 3: Try with explicit timezone
    if let Ok(dt) = DateTime::parse_from_str(&time_str, "%b %e %H:%M:%S %Y %Z") {
        return Ok(dt.to_utc());
    }
    
    Err(NylonError::ConfigError(format!(
        "Failed to parse ASN1 time: {}",
        time_str
    )))
}

/// Certificate store สำหรับเก็บข้อมูล ACME certificates
#[derive(Debug, Clone, Default)]
pub struct CertificateStore;

impl CertificateStore {
    pub fn new() -> Self {
        Self
    }
}
