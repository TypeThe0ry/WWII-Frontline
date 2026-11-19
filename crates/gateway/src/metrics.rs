use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};

pub struct Metrics {
    registry: Registry,
    requests: IntCounterVec,
    durations: HistogramVec,
    auth: IntCounterVec,
}

impl Default for Metrics {
    fn default() -> Self {
        let registry = Registry::new();
        let requests = IntCounterVec::new(
            Opts::new(
                "gateway_http_requests_total",
                "HTTP responses by route and status class",
            ),
            &["route", "status_class"],
        )
        .expect("valid HTTP counter");
        let durations = HistogramVec::new(
            HistogramOpts::new(
                "gateway_http_request_duration_seconds",
                "HTTP handler latency",
            )
            .buckets(vec![
                0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1., 5.,
            ]),
            &["route"],
        )
        .expect("valid HTTP histogram");
        let auth = IntCounterVec::new(
            Opts::new(
                "gateway_auth_attempts_total",
                "Guest login and renewal or ticket authentication outcomes",
            ),
            &["operation", "result"],
        )
        .expect("valid auth counter");
        for collector in [
            Box::new(requests.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(durations.clone()),
            Box::new(auth.clone()),
        ] {
            registry.register(collector).expect("unique metric");
        }
        Self {
            registry,
            requests,
            durations,
            auth,
        }
    }
}

impl Metrics {
    pub fn request(&self, route: &str, status: u16, elapsed_s: f64) {
        let class = match status / 100 {
            2 => "2xx",
            3 => "3xx",
            4 => "4xx",
            5 => "5xx",
            _ => "other",
        };
        self.requests.with_label_values(&[route, class]).inc();
        self.durations
            .with_label_values(&[route])
            .observe(elapsed_s);
    }

    pub fn auth(&self, operation: &'static str, result: &'static str) {
        self.auth.with_label_values(&[operation, result]).inc();
    }

    pub fn encode(&self) -> Result<(String, Vec<u8>), prometheus::Error> {
        let encoder = TextEncoder::new();
        let mut body = Vec::new();
        encoder.encode(&self.registry.gather(), &mut body)?;
        Ok((encoder.format_type().to_owned(), body))
    }
}
