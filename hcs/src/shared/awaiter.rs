use std::time::Duration;

use tokio::time::{
    Instant,
    sleep,
};

const DEFAULT_POLL_INTERVAL_MS: u64 = 1000;
const DEFAULT_TIMEOUT_SECS: u64 = 45;

/// Poll `fetch_fn` until `check_fn` returns true or timeout is reached.
pub async fn wait_for_changes<T, FutT, F, C>(
    fetch_fn: F,
    check_fn: C,
    timeout_ms: Option<u64>,
    poll_interval_ms: Option<u64>,
) -> Result<(), String>
where
    F: Fn() -> FutT,
    FutT: std::future::Future<Output = Result<T, String>>,
    C: Fn(&T) -> bool,
{
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(DEFAULT_TIMEOUT_SECS * 1000));
    let poll_interval = Duration::from_millis(poll_interval_ms.unwrap_or(DEFAULT_POLL_INTERVAL_MS));
    let start = Instant::now();

    loop {
        match fetch_fn().await {
            Ok(result) => {
                if check_fn(&result) {
                    return Ok(());
                }
            }
            Err(_) => {}
        }

        if start.elapsed() >= timeout {
            return Err(format!("Timed out waiting for changes after {}ms", timeout.as_millis()));
        }

        sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{
        AtomicUsize,
        Ordering,
    };

    use super::wait_for_changes;

    #[tokio::test]
    async fn succeeds_when_condition_met_before_timeout() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_ref = attempts.clone();
        let out = wait_for_changes(
            move || {
                let attempts_ref = attempts_ref.clone();
                async move {
                    let n = attempts_ref.fetch_add(1, Ordering::SeqCst) + 1;
                    Ok::<usize, String>(n)
                }
            },
            |n: &usize| *n >= 3,
            Some(3_000),
            Some(10),
        )
        .await;

        assert!(out.is_ok());
        assert!(attempts.load(Ordering::SeqCst) >= 3);
    }

    #[tokio::test]
    async fn times_out_when_condition_never_met() {
        let out = wait_for_changes(
            || async { Ok::<usize, String>(1) },
            |_n: &usize| false,
            Some(50),
            Some(10),
        )
        .await;

        assert!(out.is_err());
        assert!(out.err().expect("error").contains("Timed out waiting for changes"));
    }
}
