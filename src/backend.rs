use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct BackendNode {
    pub id: usize,
    pub url: String,
    pub is_healthy: AtomicBool,
    pub active_requests: AtomicUsize,
    pub avg_latency_ms: AtomicU64,
}

impl BackendNode {
    pub fn new(id: usize, url: String) -> Arc<Self> {
        Arc::new(Self {
            id,
            url,
            is_healthy: AtomicBool::new(true),
            active_requests: AtomicUsize::new(0),
            avg_latency_ms: AtomicU64::new(0),
        })
    }

    pub fn set_healthy(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::Release);
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Acquire)
    }

    pub fn increment_requests(&self) -> usize {
        self.active_requests.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn decrement_requests(&self) {
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn get_active_requests(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }

    pub fn record_latency(&self, duration_ms: u64) {
        let current = self.avg_latency_ms.load(Ordering::Relaxed);
        let new_avg = if current == 0 {
            duration_ms
        } else {
            // Exponentieller gleitender Mittelwert (EMA) mit alpha = 0.2
            ((current as f64 * 0.8) + (duration_ms as f64 * 0.2)) as u64
        };
        self.avg_latency_ms.store(new_avg, Ordering::Relaxed);
    }

    pub fn get_avg_latency(&self) -> u64 {
        self.avg_latency_ms.load(Ordering::Relaxed)
    }
}
