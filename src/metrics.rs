use std::collections::HashMap;

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder,
};

/// Content-Type для Prometheus
pub const TEXT_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// границы latency в секундах: от 100 мкс до 5 с.
pub const LATENCY_BUCKETS: [f64; 15] = [
    0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
];

pub const METRIC_LATENCY: &str = "http_request_duration_seconds";
pub const METRIC_REQUESTS: &str = "http_requests_total";

pub const LABEL_METHOD: &str = "method";
pub const LABEL_HANDLER: &str = "handler";
pub const LABEL_CODE: &str = "code";
pub const LABEL_POD: &str = "pod";
pub const LABEL_DATACENTER: &str = "dc";

/// метрики одного экземпляра приложения
#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    latency: HistogramVec,
    requests: IntCounterVec,
}

impl Metrics {
    /// создание реестра для экземпляра
    ///
    /// Использование:
    /// ```
    /// let metrics = items_service::metrics::Metrics::new("pod-1", "dc1").unwrap();
    /// metrics.observe("GET", "/items/{id}", 200, 0.01);
    /// let text = String::from_utf8(metrics.encode().unwrap()).unwrap();
    /// assert!(text.contains("http_requests_total"));
    /// ```
    pub fn new(pod_id: &str, datacenter: &str) -> Result<Self, prometheus::Error> {
        let const_labels = HashMap::from([
            (LABEL_POD.to_owned(), pod_id.to_owned()),
            (LABEL_DATACENTER.to_owned(), datacenter.to_owned()),
        ]);
        let registry = Registry::new_custom(None, Some(const_labels))?;

        let latency = HistogramVec::new(
            HistogramOpts::new(
                METRIC_LATENCY,
                "Длительность обработки HTTP-запроса в секундах",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
            &[LABEL_METHOD, LABEL_HANDLER],
        )?;

        let requests = IntCounterVec::new(
            Opts::new(METRIC_REQUESTS, "Число HTTP-ответов по кодам"),
            &[LABEL_METHOD, LABEL_HANDLER, LABEL_CODE],
        )?;

        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(requests.clone()))?;

        Ok(Self {
            registry,
            latency,
            requests,
        })
    }

    /// отслеживаем запрос
    pub fn observe(&self, method: &str, handler: &str, code: u16, duration_seconds: f64) {
        self.requests
            .with_label_values(&[method, handler, &code.to_string()])
            .inc();
        self.latency
            .with_label_values(&[method, handler])
            .observe(duration_seconds);
    }

    /// экспорт метрик в текстовом формате
    pub fn encode(&self) -> Result<Vec<u8>, prometheus::Error> {
        let mut buffer = Vec::new();
        TextEncoder::new().encode(&self.registry.gather(), &mut buffer)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exported_text() -> String {
        let metrics = Metrics::new("pod-test", "dc-test").expect("реестр метрик");
        metrics.observe("POST", "/items", 201, 0.012);
        metrics.observe("GET", "/items/{id}", 404, 0.002);

        String::from_utf8(metrics.encode().expect("экспорт метрик")).expect("utf-8")
    }

    #[test]
    fn counter_carries_method_handler_code_pod_and_dc() {
        let text = exported_text();

        for expected in [
            METRIC_REQUESTS,
            r#"method="POST""#,
            r#"handler="/items""#,
            r#"code="201""#,
            r#"pod="pod-test""#,
            r#"dc="dc-test""#,
        ] {
            assert!(
                text.contains(expected),
                "в экспорте нет {expected}:\n{text}"
            );
        }
    }

    #[test]
    fn latency_is_exported_as_histogram_buckets() {
        let text = exported_text();

        assert!(
            text.contains("http_request_duration_seconds_bucket{"),
            "{text}"
        );
        assert!(text.contains(r#"le="0.025""#), "{text}");
        assert!(text.contains(r#"le="0.0005""#), "{text}");
        assert!(text.contains(r#"le="0.0001""#), "{text}");
        assert!(text.contains("http_request_duration_seconds_sum"), "{text}");
        assert!(
            text.contains("http_request_duration_seconds_count"),
            "{text}"
        );
    }

    #[test]
    fn histogram_has_no_code_label() {
        let text = exported_text();
        let histogram_lines = text.lines().filter(|line| line.starts_with(METRIC_LATENCY));

        for line in histogram_lines {
            assert!(!line.contains("code="), "лишняя кардинальность: {line}");
        }
    }
}
