pub mod db;
pub mod error;
pub mod judge;
pub mod routes;
pub mod worker;

use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

struct JsonVisitor<'a>(&'a mut serde_json::Map<String, serde_json::Value>);

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::Value::from(value));
    }
}

/// Formats events into structured JSON with a `severity` field matching Cloud Logging.
struct CloudLoggingLayer;

impl<S: tracing::Subscriber> Layer<S> for CloudLoggingLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut fields = serde_json::Map::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        let severity = match *event.metadata().level() {
            tracing::Level::TRACE | tracing::Level::DEBUG => "DEBUG",
            tracing::Level::INFO => "INFO",
            tracing::Level::WARN => "WARNING",
            tracing::Level::ERROR => "ERROR",
        };

        let message = fields
            .remove("message")
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s),
                _ => None,
            })
            .unwrap_or_default();

        let mut log = serde_json::json!({
            "severity": severity,
            "message": message,
            "target": event.metadata().target(),
        });
        if let Some(obj) = log.as_object_mut() {
            for (k, v) in fields {
                obj.insert(k, v);
            }
        }
        if let Ok(line) = serde_json::to_string(&log) {
            println!("{line}");
        }
    }
}

pub fn resolve_filter_from(rust_log: Option<&str>, is_prod: bool) -> String {
    if let Some(rl) = rust_log {
        return rl.to_string();
    }

    if is_prod {
        "info,tokio_postgres=warn".to_string()
    } else {
        "debug,tokio_postgres=warn,tower_http=debug,axum::rejection=trace".to_string()
    }
}

fn resolve_filter(is_prod: bool) -> String {
    let rust_log = env::var("RUST_LOG").ok();
    resolve_filter_from(rust_log.as_deref(), is_prod)
}

pub fn init_tracing() {
    let app_env = env::var("APP_ENV")
        .or_else(|_| env::var("ENV"))
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase();
    let is_prod = app_env == "production" || app_env == "prod";

    let filter_str = resolve_filter(is_prod);
    let env_filter = EnvFilter::try_new(&filter_str)
        .unwrap_or_else(|_| EnvFilter::new("info,tokio_postgres=warn"));

    if is_prod {
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(CloudLoggingLayer)
            .try_init();
    } else {
        let _ = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_filter_dev_default() {
        assert_eq!(
            resolve_filter_from(None, false),
            "debug,tokio_postgres=warn,tower_http=debug,axum::rejection=trace"
        );
    }

    #[test]
    fn test_resolve_filter_prod_default() {
        assert_eq!(resolve_filter_from(None, true), "info,tokio_postgres=warn");
    }

    #[test]
    fn test_resolve_filter_rust_log_precedence() {
        assert_eq!(
            resolve_filter_from(Some("trace,my_crate=info"), true),
            "trace,my_crate=info"
        );
    }
}
