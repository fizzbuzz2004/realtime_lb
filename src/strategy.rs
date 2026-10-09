use crate::backend::BackendNode;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyType {
    RoundRobin = 0,
    LeastConnections = 1,
    LatencyAware = 2,
}

impl StrategyType {
    pub fn from_u8(val: u8) -> Self {
        match val {
            1 => StrategyType::LeastConnections,
            2 => StrategyType::LatencyAware,
            _ => StrategyType::RoundRobin,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StrategyType::RoundRobin => "Round-Robin",
            StrategyType::LeastConnections => "Least-Connections",
            StrategyType::LatencyAware => "Latency-Aware",
        }
    }
}

pub struct LoadBalancingStrategy {
    current_strategy: AtomicU8,
    rr_index: AtomicUsize,
}

impl LoadBalancingStrategy {
    pub fn new(initial: StrategyType) -> Arc<Self> {
        Arc::new(Self {
            current_strategy: AtomicU8::new(initial as u8),
            rr_index: AtomicUsize::new(0),
        })
    }

    pub fn get_current(&self) -> StrategyType {
        StrategyType::from_u8(self.current_strategy.load(Ordering::Relaxed))
    }

    pub fn set_strategy(&self, strategy: StrategyType) {
        let old = self.get_current();
        if old != strategy {
            self.current_strategy.store(strategy as u8, Ordering::Relaxed);
            info!(
                "Adaptive Switcher: Algorithmus gewechselt von {} zu {}",
                old.as_str(),
                strategy.as_str()
            );
        }
    }

    pub fn select_backend<'a>(&self, backends: &'a [Arc<BackendNode>]) -> Option<Arc<BackendNode>> {
        let healthy_nodes: Vec<&Arc<BackendNode>> = backends
            .iter()
            .filter(|node| node.is_healthy())
            .collect();

        if healthy_nodes.is_empty() {
            return None;
        }

        match self.get_current() {
            StrategyType::RoundRobin => {
                let idx = self.rr_index.fetch_add(1, Ordering::Relaxed) % healthy_nodes.len();
                Some(Arc::clone(healthy_nodes[idx]))
            }
            StrategyType::LeastConnections => {
                healthy_nodes
                    .into_iter()
                    .min_by_key(|node| node.get_active_requests())
                    .cloned()
            }
            StrategyType::LatencyAware => {
                healthy_nodes
                    .into_iter()
                    .min_by_key(|node| node.get_avg_latency())
                    .cloned()
            }
        }
    }
}
