//! INV-C adaptive GIN count batching (SPEC-107 R2 / SPEC-149).
//!
//! Batches start at [`edgequake_storage::SOURCE_PREFIX_BATCH_LIMIT`] (LAW-H1).
//! Startup keeps the list-path `SOURCE_COUNT_STATEMENT_TIMEOUT_MS` kill
//! (LAW-H2). The hourly monitor raises that kill so a batch can finish
//! under ingest instead of being cancelled and retried. A batch killed by
//! its timeout is split in half and retried, so
//! heavy documents shrink the batch instead of skipping the whole sample. A
//! single prefix that still times out is skipped (not drift), and the whole
//! loop stops at a wall-clock budget. Startup uses [`INV_C_STARTUP_BUDGET`]
//! (boot awaits the inspector). The hourly monitor uses
//! [`INV_C_MONITOR_BUDGET`] so ingest load cannot skip the sample.

use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};

/// Wall-clock cap for INV-C counts at startup. Boot awaits the inspector.
pub(super) const INV_C_STARTUP_BUDGET: Duration = Duration::from_secs(2);

/// Wall-clock cap for the hourly monitor. It runs in the background, including
/// while ingest is writing, so it can finish the sample after batches shrink.
pub(super) const INV_C_MONITOR_BUDGET: Duration = Duration::from_secs(30);

/// Per-statement kill for the hourly monitor.
///
/// The list-path kill ([`edgequake_storage::SOURCE_COUNT_STATEMENT_TIMEOUT_MS`],
/// 300ms) stays on startup: under ingest a 32-prefix batch misses 300ms, five
/// halvings burn the 2s wall, and the rest of the sample is skipped
/// (observed: counted=10, skipped=40). Two seconds lets a heavy batch finish
/// instead of being cancelled and retried.
pub(super) const INV_C_MONITOR_STATEMENT_TIMEOUT_MS: u32 = 2_000;

/// Budgets for one INV-C count pass.
#[derive(Debug, Clone, Copy)]
pub(super) struct InvCLimits {
    pub wall: Duration,
    pub statement_timeout_ms: u32,
}

/// Outcome of one count round-trip.
#[derive(Debug)]
pub(super) enum BatchError {
    /// SQLSTATE 57014 — the statement hit its `SET LOCAL statement_timeout`.
    Timeout(String),
    /// Any other failure (connection, missing graph, …): not retried.
    Failed(String),
}

/// Counts for the prefixes that could be evaluated.
#[derive(Debug, Default)]
pub(super) struct AdaptiveCounts {
    pub counts: HashMap<String, i64>,
    /// Prefixes left unevaluated (single-prefix timeout, budget, or failure).
    pub skipped: usize,
    pub splits: usize,
}

/// Classify a count query error; only statement timeouts are retryable.
pub(super) fn classify_count_error(err: &sqlx::Error) -> BatchError {
    let timed_out = err
        .as_database_error()
        .and_then(|db| db.code())
        .is_some_and(|code| code == "57014");
    let msg = format!("INV-C GIN count failed: {err}");
    if timed_out {
        BatchError::Timeout(msg)
    } else {
        BatchError::Failed(msg)
    }
}

/// Run `run` over `prefixes`, halving any batch that times out.
///
/// Errors only when nothing could be counted, so INV-C reports a visible
/// skip instead of a silent green result.
pub(super) async fn count_adaptively<F, Fut>(
    prefixes: &[String],
    batch_limit: usize,
    budget: Duration,
    mut run: F,
) -> Result<AdaptiveCounts, String>
where
    F: FnMut(Vec<String>) -> Fut,
    Fut: Future<Output = Result<HashMap<String, i64>, BatchError>>,
{
    let started = Instant::now();
    let mut out = AdaptiveCounts::default();
    let mut last_error: Option<String> = None;
    // The halved size sticks: the rest of the sample is usually as heavy, and
    // retrying full-size batches would burn a statement timeout each time.
    let mut limit = batch_limit.max(1);
    let mut cursor = 0;

    while cursor < prefixes.len() {
        if started.elapsed() >= budget {
            out.skipped += prefixes.len() - cursor;
            last_error.get_or_insert_with(|| format!("INV-C count budget {budget:?} exhausted"));
            break;
        }
        let batch = &prefixes[cursor..(cursor + limit).min(prefixes.len())];
        match run(batch.to_vec()).await {
            Ok(partial) => {
                out.counts.extend(partial);
                cursor += batch.len();
            }
            Err(BatchError::Timeout(e)) if batch.len() > 1 => {
                limit = batch.len() / 2;
                out.splits += 1;
                last_error = Some(e);
            }
            Err(BatchError::Timeout(e)) => {
                out.skipped += 1;
                cursor += 1;
                last_error = Some(e);
            }
            Err(BatchError::Failed(e)) => {
                // SPEC-107 R2 EC-07: keep earlier batches, stop on hard failure.
                out.skipped += prefixes.len() - cursor;
                last_error = Some(e);
                break;
            }
        }
    }

    if out.counts.is_empty() && !prefixes.is_empty() {
        return Err(last_error.unwrap_or_else(|| "INV-C count returned no rows".to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefixes(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("doc-{i}-chunk-")).collect()
    }

    /// Fake runner: times out when a batch is larger than `max_ok` or
    /// contains a prefix listed in `always_slow`.
    async fn fake(
        batch: Vec<String>,
        max_ok: usize,
        always_slow: &[&str],
    ) -> Result<HashMap<String, i64>, BatchError> {
        if batch.len() > max_ok || batch.iter().any(|p| always_slow.contains(&p.as_str())) {
            return Err(BatchError::Timeout("57014".into()));
        }
        Ok(batch.into_iter().map(|p| (p, 1)).collect())
    }

    #[tokio::test]
    async fn timed_out_batches_are_halved_and_the_size_sticks() {
        let input = prefixes(50);
        let mut sizes = Vec::new();
        let got = count_adaptively(&input, 32, INV_C_STARTUP_BUDGET, |b| {
            sizes.push(b.len());
            fake(b, 10, &[])
        })
        .await
        .expect("counts");
        assert_eq!(got.counts.len(), 50);
        assert_eq!(got.skipped, 0);
        assert_eq!(got.splits, 2);
        assert_eq!(sizes, vec![32, 16, 8, 8, 8, 8, 8, 8, 2]);
    }

    #[tokio::test]
    async fn single_oversized_prefix_is_skipped_not_fatal() {
        let input = prefixes(4);
        let got = count_adaptively(&input, 32, INV_C_STARTUP_BUDGET, |b| {
            fake(b, 32, &["doc-2-chunk-"])
        })
        .await
        .expect("partial counts");
        assert_eq!(got.skipped, 1);
        assert_eq!(got.counts.len(), 3);
        assert!(!got.counts.contains_key("doc-2-chunk-"));
    }

    #[tokio::test]
    async fn hard_failure_first_is_an_error_and_later_keeps_partial() {
        let input = prefixes(40);
        let err = count_adaptively(&input, 32, INV_C_STARTUP_BUDGET, |_| async {
            Err(BatchError::Failed("42P01".into()))
        })
        .await
        .expect_err("nothing counted");
        assert!(err.contains("42P01"));

        let mut calls = 0;
        let got = count_adaptively(&input, 32, INV_C_STARTUP_BUDGET, |b| {
            calls += 1;
            let first = calls == 1;
            async move {
                if first {
                    Ok(b.into_iter().map(|p| (p, 1)).collect())
                } else {
                    Err(BatchError::Failed("conn reset".into()))
                }
            }
        })
        .await
        .expect("partial");
        assert_eq!(got.counts.len(), 32);
        assert_eq!(got.skipped, 8);
    }

    #[tokio::test]
    async fn exhausted_budget_is_a_visible_error() {
        let input = prefixes(3);
        let err = count_adaptively(&input, 32, Duration::ZERO, |b| fake(b, 32, &[]))
            .await
            .expect_err("budget");
        assert!(err.contains("budget"));
    }

    #[tokio::test]
    async fn all_single_prefix_timeouts_report_the_timeout() {
        let input = prefixes(2);
        let err = count_adaptively(&input, 32, INV_C_STARTUP_BUDGET, |b| fake(b, 0, &[]))
            .await
            .expect_err("nothing fits");
        assert!(err.contains("57014"));
    }

    #[test]
    fn monitor_budget_outlasts_startup_and_the_list_path_kill() {
        assert!(INV_C_MONITOR_BUDGET > INV_C_STARTUP_BUDGET);
        const {
            assert!(INV_C_MONITOR_STATEMENT_TIMEOUT_MS > 300);
        }
        // Five halvings from 32 must still leave time to count the sample.
        let five_kills = Duration::from_millis(u64::from(INV_C_MONITOR_STATEMENT_TIMEOUT_MS) * 5);
        assert!(INV_C_MONITOR_BUDGET > five_kills + Duration::from_secs(10));
    }

    async fn slow_single(batch: Vec<String>) -> Result<HashMap<String, i64>, BatchError> {
        tokio::time::sleep(Duration::from_millis(30)).await;
        if batch.len() > 1 {
            Err(BatchError::Timeout("57014".into()))
        } else {
            Ok(batch.into_iter().map(|p| (p, 1)).collect())
        }
    }

    /// A short wall dies while statement timeouts are still discovering a batch
    /// size; a longer wall finishes the same sample.
    #[tokio::test]
    async fn short_wall_skips_the_sample_monitor_wall_finishes_it() {
        let input = prefixes(6);
        let short = count_adaptively(&input, 4, Duration::from_millis(70), slow_single).await;
        let stopped_early = match &short {
            Ok(got) => got.skipped > 0 && got.counts.len() < input.len(),
            Err(e) => e.contains("budget"),
        };
        assert!(
            stopped_early,
            "startup-sized wall must stop early: {short:?}"
        );

        let full = count_adaptively(&input, 4, Duration::from_secs(3), slow_single)
            .await
            .expect("full");
        assert_eq!(full.counts.len(), input.len());
        assert_eq!(full.skipped, 0);
    }

    #[test]
    fn only_sqlstate_57014_is_retryable() {
        let other = classify_count_error(&sqlx::Error::RowNotFound);
        assert!(matches!(other, BatchError::Failed(_)));
    }

    #[tokio::test]
    async fn real_statement_timeout_classifies_as_retryable() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("SKIP: no DATABASE_URL");
            return;
        };
        let pool = sqlx::PgPool::connect(&url).await.expect("connect");
        let mut tx = pool.begin().await.expect("begin");
        sqlx::query("SET LOCAL statement_timeout = '10ms'")
            .execute(&mut *tx)
            .await
            .expect("set timeout");
        let err = sqlx::query("SELECT pg_sleep(1)")
            .execute(&mut *tx)
            .await
            .expect_err("must hit statement_timeout");
        assert!(matches!(classify_count_error(&err), BatchError::Timeout(_)));
    }
}
