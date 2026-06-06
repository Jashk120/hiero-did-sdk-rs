use std::future::Future;
use tokio::time::{Duration, Instant, sleep};

/// Polls `f` every `interval_ms` milliseconds until it returns `Some(T)` or `timeout_secs` elapses.
/// Returns `Some(T)` on success, `None` on timeout.
pub async fn poll_until<F, Fut, T>(
    mut f: F,
    timeout_secs: u64,
    interval_ms: u64,
) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Some(result) = f().await {
            return Some(result);
        }
        if Instant::now() >= deadline {
            return None;
        }
        sleep(Duration::from_millis(interval_ms)).await;
    }
}