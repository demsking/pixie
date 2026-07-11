use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logging() {
  let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new("pixie_core=info,pixie_app=info,warn"));

  tracing_subscriber::registry()
    .with(filter)
    .with(fmt::layer().compact())
    .init();
}
