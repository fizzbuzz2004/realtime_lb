use crate::backend::BackendNode;
use crate::strategy::LoadBalancingStrategy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub static TOTAL_REQUESTS: AtomicU64 = AtomicU64::new(0);

pub fn generate_prometheus_metrics(
    backends: &[Arc<BackendNode>],
    strategy: &Arc<LoadBalancingStrategy>,
) -> String {
    let mut out = String::new();

    // 1. Gesamtzahl verarbeiteter Anfragen
    out.push_str("# HELP lb_requests_total Total number of HTTP requests processed by the load balancer.\n");
    out.push_str("# TYPE lb_requests_total counter\n");
    out.push_str(&format!(
        "lb_requests_total {}\n\n",
        TOTAL_REQUESTS.load(Ordering::Relaxed)
    ));

    // 2. Aktuell aktiver Routing-Algorithmus (0=RoundRobin, 1=LeastConnections, 2=LatencyAware)
    out.push_str("# HELP lb_current_strategy Current load balancing strategy (0=RoundRobin, 1=LeastConnections, 2=LatencyAware).\n");
    out.push_str("# TYPE lb_current_strategy gauge\n");
    out.push_str(&format!(
        "lb_current_strategy {}\n\n",
        strategy.get_current() as u8
    ));

    // 3. Health Status pro Backend (1 = Healthy, 0 = Unhealthy)
    out.push_str("# HELP lb_backend_healthy Backend node health status (1 = healthy, 0 = unhealthy).\n");
    out.push_str("# TYPE lb_backend_healthy gauge\n");
    for b in backends {
        let healthy_val = if b.is_healthy() { 1 } else { 0 };
        out.push_str(&format!(
            "lb_backend_healthy{{backend_id=\"{}\",url=\"{}\"}} {}\n",
            b.id, b.url, healthy_val
        ));
    }
    out.push('\n');

    // 4. Aktive Verbindungen pro Backend
    out.push_str("# HELP lb_backend_active_requests Current number of active in-flight requests per backend.\n");
    out.push_str("# TYPE lb_backend_active_requests gauge\n");
    for b in backends {
        out.push_str(&format!(
            "lb_backend_active_requests{{backend_id=\"{}\",url=\"{}\"}} {}\n",
            b.id, b.url, b.get_active_requests()
        ));
    }
    out.push('\n');

    // 5. Exponentiell gewichteter Latenz-Mittelwert (EMA) in ms
    out.push_str("# HELP lb_backend_latency_ms Exponential moving average latency in milliseconds per backend.\n");
    out.push_str("# TYPE lb_backend_latency_ms gauge\n");
    for b in backends {
        out.push_str(&format!(
            "lb_backend_latency_ms{{backend_id=\"{}\",url=\"{}\"}} {}\n",
            b.id, b.url, b.get_avg_latency()
        ));
    }
    out.push('\n');

    out
}
