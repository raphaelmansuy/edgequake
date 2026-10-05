//! Optional SQLx execution evidence. Bound values are not logged by SQLx.
//! Captures completion/drop timings, which may include failed or cancelled
//! statements: this is measurement evidence, not a correctness assertion.
#![allow(dead_code)]

pub fn initialize() {
    let Ok(path) = std::env::var("EQ_QUERY_EXECUTION_LOG") else {
        return;
    };
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open query execution log");
        tracing_subscriber::fmt()
            .json()
            .with_env_filter("sqlx::query=debug")
            .with_current_span(false)
            .with_span_list(false)
            .with_writer(std::sync::Mutex::new(file))
            .try_init()
            .expect("query capture must initialize before another tracing subscriber");
    });
}
