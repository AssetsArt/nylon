use chrono::{DateTime, Utc};
use nylon_error::NylonError;
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
        let expires_at = parse_asn1_time(not_after.to_string().as_str())?;
        let issued_at = parse_asn1_time(not_before.to_string().as_str())?;

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

/// แปลง ASN1Time string เป็น DateTime<Utc>
fn parse_asn1_time(time_str: &str) -> Result<DateTime<Utc>, NylonError> {
    // ASN1 time format: "Jan 1 12:00:00 2025 GMT"
    use chrono::NaiveDateTime;

    // พยายาม parse ด้วยรูปแบบ GMT ก่อน
    if let Ok(parsed) = chrono::DateTime::parse_from_str(time_str, "%b %d %H:%M:%S %Y GMT") {
        return Ok(parsed.to_utc());
    }

    // ถ้าไม่ได้ ลองรูปแบบอื่น
    let naive = NaiveDateTime::parse_from_str(time_str, "%b %d %H:%M:%S %Y")
        .map_err(|e| NylonError::ConfigError(format!("Failed to parse time: {}", e)))?;

    Ok(chrono::DateTime::<Utc>::from_naive_utc_and_offset(
        naive, Utc,
    ))
}

/// Certificate store สำหรับเก็บข้อมูล ACME certificates
#[derive(Debug, Clone, Default)]
pub struct CertificateStore;

impl CertificateStore {
    pub fn new() -> Self {
        Self
    }
}
