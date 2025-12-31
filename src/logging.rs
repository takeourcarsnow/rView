use tracing_subscriber::EnvFilter;

/// Initialize tracing and bridge `log` to `tracing`.
/// Calling this multiple times is safe (subsequent attempts are ignored where possible).
pub fn init_tracing(enable_debug: bool) {
    // Bridge `log` records into `tracing` so existing `log` macros are captured
    let _ = tracing_log::LogTracer::init();

    // Prefer explicit debug flag, otherwise fall back to RUST_LOG or default to warn
    let env_filter = if enable_debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    // Use try_init so calling this multiple times (e.g., in tests) doesn't panic
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_names(false)
        .try_init()
        .ok();
}
