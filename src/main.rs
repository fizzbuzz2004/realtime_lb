mod backend;
mod health;
mod metrics;
mod proxy;
mod strategy;

use backend::BackendNode;
use health::HealthChecker;
use proxy::ProxyServer;
use strategy::{LoadBalancingStrategy, StrategyType};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    info!("Starte Adaptive Real-Time Load Balancer in Rust...");

    // 1. Upstream-Backend Knoten
    let backend_urls = vec![
        "http://127.0.0.1:8081".to_string(),
        "http://127.0.0.1:8082".to_string(),
        "http://127.0.0.1:8083".to_string(),
    ];

    let backends: Vec<Arc<BackendNode>> = backend_urls
        .into_iter()
        .enumerate()
        .map(|(idx, url)| BackendNode::new(idx + 1, url))
        .collect();

    // 2. Initialen Algorithmus setzen (Standard: Round-Robin)
    let strategy = LoadBalancingStrategy::new(StrategyType::RoundRobin);

    // 3. Health Monitor im Hintergrund starten
    let health_checker = HealthChecker::new(backends.clone(), Duration::from_secs(5));
    tokio::spawn(async move {
        health_checker.start_monitoring().await;
    });

    // 4. Adaptiver Echtzeit-Metrik-Switcher (Regelkreis)
    let backends_for_adaptive = backends.clone();
    let strategy_for_adaptive = strategy.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(10)).await;

            let total_active_conns: usize = backends_for_adaptive
                .iter()
                .map(|b| b.get_active_requests())
                .sum();

            let avg_latency: u64 = {
                let healthy: Vec<_> = backends_for_adaptive
                    .iter()
                    .filter(|b| b.is_healthy())
                    .collect();
                if healthy.is_empty() {
                    0
                } else {
                    healthy.iter().map(|b| b.get_avg_latency()).sum::<u64>() / healthy.len() as u64
                }
            };

            // Dynamische Anpassung basierend auf Systemmetriken
            if total_active_conns > 50 {
                // Hohe Concurrency -> Least Connections wählen
                strategy_for_adaptive.set_strategy(StrategyType::LeastConnections);
            } else if avg_latency > 200 {
                // Hohe Latenzunterschiede -> Latency Aware wählen
                strategy_for_adaptive.set_strategy(StrategyType::LatencyAware);
            } else {
                // Normale Last -> Gleichmäßiges Round-Robin
                strategy_for_adaptive.set_strategy(StrategyType::RoundRobin);
            }
        }
    });

    // 5. Proxy Server starten
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let server = ProxyServer::new(addr, backends, strategy);
    server.run().await?;

    Ok(())
}
