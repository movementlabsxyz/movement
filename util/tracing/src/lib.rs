//! Tracing setup for Movement services.
//!
//! Exporting of tracing data via [OpenTelemetry] is optionally supported
//! by setting "movement_telemetry" as the target in tracing spans and events.
//!
//! [OpenTelemetry]: https://opentelemetry.io/

mod config;
mod telemetry;
mod tracing;

pub use config::Config;
pub use telemetry::ScopeGuard;
pub use tracing::init_tracing_subscriber;
