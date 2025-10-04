use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use dashmap::DashMap;
use chrono::{DateTime, Utc};

/// ACME metrics สำหรับ monitoring
#[derive(Debug, Clone)]
pub struct AcmeMetrics {
    /// จำนวนครั้งที่ issue certificate สำเร็จ
    pub issuance_success: Arc<AtomicU64>,
    /// จำนวนครั้งที่ issue certificate ล้มเหลว
    pub issuance_failure: Arc<AtomicU64>,
    /// จำนวนครั้งที่ renew certificate สำเร็จ
    pub renewal_success: Arc<AtomicU64>,
    /// จำนวนครั้งที่ renew certificate ล้มเหลว
    pub renewal_failure: Arc<AtomicU64>,
    /// จำนวนครั้งที่ challenge validation สำเร็จ
    pub challenge_success: Arc<AtomicU64>,
    /// จำนวนครั้งที่ challenge validation ล้มเหลว
    pub challenge_failure: Arc<AtomicU64>,
    /// Domain-specific metrics
    pub domain_metrics: Arc<DashMap<String, DomainMetrics>>,
}

/// Metrics สำหรับแต่ละ domain
#[derive(Debug, Clone)]
pub struct DomainMetrics {
    pub domain: String,
    pub last_issuance: Option<DateTime<Utc>>,
    pub last_renewal: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
    pub failure_count: u32,
    pub days_until_expiry: i64,
}

impl Default for AcmeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl AcmeMetrics {
    pub fn new() -> Self {
        Self {
            issuance_success: Arc::new(AtomicU64::new(0)),
            issuance_failure: Arc::new(AtomicU64::new(0)),
            renewal_success: Arc::new(AtomicU64::new(0)),
            renewal_failure: Arc::new(AtomicU64::new(0)),
            challenge_success: Arc::new(AtomicU64::new(0)),
            challenge_failure: Arc::new(AtomicU64::new(0)),
            domain_metrics: Arc::new(DashMap::new()),
        }
    }

    /// บันทึก issuance success
    pub fn record_issuance_success(&self, domain: &str) {
        self.issuance_success.fetch_add(1, Ordering::Relaxed);
        
        let mut metrics = self.domain_metrics
            .entry(domain.to_string())
            .or_insert_with(|| DomainMetrics {
                domain: domain.to_string(),
                last_issuance: None,
                last_renewal: None,
                last_failure: None,
                failure_count: 0,
                days_until_expiry: 0,
            });
        
        metrics.last_issuance = Some(Utc::now());
        metrics.failure_count = 0; // Reset failure count on success
    }

    /// บันทึก issuance failure
    pub fn record_issuance_failure(&self, domain: &str) {
        self.issuance_failure.fetch_add(1, Ordering::Relaxed);
        
        let mut metrics = self.domain_metrics
            .entry(domain.to_string())
            .or_insert_with(|| DomainMetrics {
                domain: domain.to_string(),
                last_issuance: None,
                last_renewal: None,
                last_failure: None,
                failure_count: 0,
                days_until_expiry: 0,
            });
        
        metrics.last_failure = Some(Utc::now());
        metrics.failure_count += 1;
    }

    /// บันทึก renewal success
    pub fn record_renewal_success(&self, domain: &str) {
        self.renewal_success.fetch_add(1, Ordering::Relaxed);
        
        let mut metrics = self.domain_metrics
            .entry(domain.to_string())
            .or_insert_with(|| DomainMetrics {
                domain: domain.to_string(),
                last_issuance: None,
                last_renewal: None,
                last_failure: None,
                failure_count: 0,
                days_until_expiry: 0,
            });
        
        metrics.last_renewal = Some(Utc::now());
        metrics.failure_count = 0; // Reset failure count on success
    }

    /// บันทึก renewal failure
    pub fn record_renewal_failure(&self, domain: &str) {
        self.renewal_failure.fetch_add(1, Ordering::Relaxed);
        
        let mut metrics = self.domain_metrics
            .entry(domain.to_string())
            .or_insert_with(|| DomainMetrics {
                domain: domain.to_string(),
                last_issuance: None,
                last_renewal: None,
                last_failure: None,
                failure_count: 0,
                days_until_expiry: 0,
            });
        
        metrics.last_failure = Some(Utc::now());
        metrics.failure_count += 1;
    }

    /// อัพเดท days until expiry
    pub fn update_days_until_expiry(&self, domain: &str, days: i64) {
        let mut metrics = self.domain_metrics
            .entry(domain.to_string())
            .or_insert_with(|| DomainMetrics {
                domain: domain.to_string(),
                last_issuance: None,
                last_renewal: None,
                last_failure: None,
                failure_count: 0,
                days_until_expiry: days,
            });
        
        metrics.days_until_expiry = days;
    }

    /// ดึง metrics ทั้งหมด
    pub fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            issuance_success: self.issuance_success.load(Ordering::Relaxed),
            issuance_failure: self.issuance_failure.load(Ordering::Relaxed),
            renewal_success: self.renewal_success.load(Ordering::Relaxed),
            renewal_failure: self.renewal_failure.load(Ordering::Relaxed),
            challenge_success: self.challenge_success.load(Ordering::Relaxed),
            challenge_failure: self.challenge_failure.load(Ordering::Relaxed),
            domain_count: self.domain_metrics.len(),
        }
    }
}

/// สรุป metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub issuance_success: u64,
    pub issuance_failure: u64,
    pub renewal_success: u64,
    pub renewal_failure: u64,
    pub challenge_success: u64,
    pub challenge_failure: u64,
    pub domain_count: usize,
}

