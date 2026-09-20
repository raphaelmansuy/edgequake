//! Deadline-bounded eventual assertions for provider-backed tests.

use std::future::Future;
use std::time::{Duration, Instant};

/// Poll an asynchronous predicate until it succeeds or the monotonic deadline
/// expires. Predicate errors fail immediately; sleeps never establish success.
pub async fn eventually<F, Fut>(
    timeout: Duration,
    interval: Duration,
    mut predicate: F,
) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if predicate().await? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "provider_access_eventual_predicate_timeout_ms:{}",
                timeout.as_millis()
            ));
        }
        tokio::time::sleep(interval).await;
    }
}

pub fn assert_contains_no_forbidden_sentinel(value: &str, forbidden: &[&str]) {
    for sentinel in forbidden {
        assert!(
            !value.contains(sentinel),
            "provider-access response contained forbidden sentinel {sentinel}"
        );
    }
}
