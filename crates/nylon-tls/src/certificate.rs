use chrono::{DateTime, Utc, TimeZone};
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

/// แปลง ASN1Time เป็น DateTime<Utc> โดยใช้ OpenSSL's diff method
fn asn1_time_to_datetime(asn1_time: &Asn1TimeRef) -> Result<DateTime<Utc>, NylonError> {
    // สร้าง reference time (Unix epoch)
    let epoch = openssl::asn1::Asn1Time::from_unix(0)
        .map_err(|e| NylonError::ConfigError(format!("Failed to create epoch time: {}", e)))?;
    
    // คำนวณ diff เป็นวินาที
    let diff = asn1_time
        .diff(&epoch)
        .map_err(|e| NylonError::ConfigError(format!("Failed to calculate time diff: {}", e)))?;
    
    // แปลง diff เป็น timestamp
    let days_in_seconds = diff.days as i64 * 86400;
    let total_seconds = days_in_seconds + diff.secs as i64;
    
    // สร้าง DateTime จาก timestamp
    Utc.timestamp_opt(total_seconds, 0)
        .single()
        .ok_or_else(|| NylonError::ConfigError("Invalid timestamp".to_string()))
}

/// Certificate store สำหรับเก็บข้อมูล ACME certificates
#[derive(Debug, Clone, Default)]
pub struct CertificateStore;

impl CertificateStore {
    pub fn new() -> Self {
        Self
    }
}
