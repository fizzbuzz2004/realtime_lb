use crate::backend::BackendNode;
use crate::metrics;
use crate::strategy::LoadBalancingStrategy;
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct ProxyServer {
    listen_addr: SocketAddr,
    backends: Vec<Arc<BackendNode>>,
    strategy: Arc<LoadBalancingStrategy>,
}

impl ProxyServer {
    pub fn new(
        listen_addr: SocketAddr,
        backends: Vec<Arc<BackendNode>>,
        strategy: Arc<LoadBalancingStrategy>,
    ) -> Self {
        Self {
            listen_addr,
            backends,
            strategy,
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        info!("Real-Time Load Balancer läuft auf http://{}", self.listen_addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let backends = self.backends.clone();
            let strategy = self.strategy.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req: Request<Incoming>| {
                    let backends = backends.clone();
                    let strategy = strategy.clone();
                    async move { Self::handle_request(req, &backends, &strategy).await }
                });

                if let Err(err) = auto::Builder::new(TokioExecutor::new())
                    .serve_connection(io, service)
                    .await
                {
                    error!("Verbindungsfehler: {:?}", err);
                }
            });
        }
    }

    async fn handle_request(
        req: Request<Incoming>,
        backends: &[Arc<BackendNode>],
        strategy: &Arc<LoadBalancingStrategy>,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        // 1. Prometheus /metrics Endpoint abfangen
        if req.uri().path() == "/metrics" {
            let metrics_payload = metrics::generate_prometheus_metrics(backends, strategy);
            let mut resp = Response::new(
                Full::new(Bytes::from(metrics_payload))
                    .map_err(|e| match e {})
                    .boxed(),
            );
            resp.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                hyper::header::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
            );
            return Ok(resp);
        }

        // Gesamtzähler verarbeiteter Requests erhöhen
        metrics::TOTAL_REQUESTS.fetch_add(1, Ordering::Relaxed);

        // 2. Upstream Backend Auswahl & Weiterleitung
        let target_backend = match strategy.select_backend(backends) {
            Some(backend) => backend,
            None => {
                error!("Kein gesunder Upstream-Server verfügbar!");
                let mut resp = Response::new(
                    Full::new(Bytes::from("503 Service Unavailable: Keine gesunden Backends verfügbar"))
                        .map_err(|e| match e {})
                        .boxed(),
                );
                *resp.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
                return Ok(resp);
            }
        };

        target_backend.increment_requests();
        let start_time = Instant::now();

        let req_method = req.method().clone();
        let req_uri = req.uri().clone();
        let req_headers = req.headers().clone();

        let body_bytes = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => {
                target_backend.decrement_requests();
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(
                        Full::new(Bytes::from("Fehlerhafter Request Body"))
                            .map_err(|e| match e {})
                            .boxed(),
                    )
                    .unwrap());
            }
        };

        let target_uri_str = format!("{}{}", target_backend.url.trim_end_matches('/'), req_uri);
        let client = reqwest::Client::new();

        let mut out_req = client.request(req_method, &target_uri_str);
        for (key, value) in req_headers.iter() {
            if key != hyper::header::HOST {
                out_req = out_req.header(key, value);
            }
        }
        out_req = out_req.body(body_bytes);

        let result = out_req.send().await;
        let elapsed_ms = start_time.elapsed().as_millis() as u64;

        target_backend.decrement_requests();
        target_backend.record_latency(elapsed_ms);

        match result {
            Ok(upstream_resp) => {
                let status = upstream_resp.status();
                let resp_headers = upstream_resp.headers().clone();

                let resp_bytes = match upstream_resp.bytes().await {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Fehler beim Lesen der Antwort vom Backend: {}", e);
                        return Ok(Response::builder()
                            .status(StatusCode::BAD_GATEWAY)
                            .body(
                                Full::new(Bytes::from("502 Bad Gateway"))
                                    .map_err(|e| match e {})
                                    .boxed(),
                            )
                            .unwrap());
                    }
                };

                let mut builder = Response::builder().status(status);
                for (key, value) in resp_headers.iter() {
                    builder = builder.header(key, value);
                }

                Ok(builder
                    .body(Full::new(resp_bytes).map_err(|e| match e {}).boxed())
                    .unwrap())
            }
            Err(err) => {
                error!(
                    "Upstream Proxy-Fehler zu Backend {} ({}): {}",
                    target_backend.id, target_backend.url, err
                );
                target_backend.set_healthy(false);

                Ok(Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(
                        Full::new(Bytes::from("502 Bad Gateway - Backend Verbindungsfehler"))
                            .map_err(|e| match e {})
                            .boxed(),
                    )
                    .unwrap())
            }
        }
    }
}
