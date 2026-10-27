use tracing_subscriber::{EnvFilter, fmt};

use crate::config::LogFormat;

const DEFAULT_FILTER: &str = "todo_backend=info,tower_http=info";

/// Installs the global tracing subscriber. Filtering is controlled by `RUST_LOG`.
pub fn init(format: &LogFormat) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));
    let builder = fmt().with_env_filter(filter);

    match format {
        LogFormat::Pretty => builder.init(),
        LogFormat::Json => builder.json().init(),
    }
}
