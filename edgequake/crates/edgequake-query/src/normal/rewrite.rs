//! Query rewriting for a Normal workspace.
//!
//! One short LLM call expands the question before the dense and BM25 legs.
//! It is on by default and always fail-open: a timeout, an error, or a
//! completion that would poison retrieval leaves the original question in
//! place. The answer prompt still sees the user's question; only retrieval
//! uses the rewrite.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use edgequake_llm::traits::LLMProvider;

/// `EDGEQUAKE_NORMAL_QUERY_REWRITE`. Default on. `0` / `false` / `off` / `no` disables.
pub const REWRITE_ENV: &str = "EDGEQUAKE_NORMAL_QUERY_REWRITE";
/// `EDGEQUAKE_NORMAL_REWRITE_TIMEOUT_MS`. Default 2000.
pub const TIMEOUT_ENV: &str = "EDGEQUAKE_NORMAL_REWRITE_TIMEOUT_MS";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);
pub const MAX_REWRITE_CHARS: usize = 400;
const CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Copy)]
pub struct RewritePolicy {
    pub enabled: bool,
    pub timeout: Duration,
}

impl RewritePolicy {
    pub fn from_env() -> Self {
        Self {
            enabled: rewrite_enabled(std::env::var(REWRITE_ENV).ok().as_deref()),
            timeout: rewrite_timeout(std::env::var(TIMEOUT_ENV).ok().as_deref()),
        }
    }
}

/// What retrieval should embed and send to BM25, plus why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteOutcome {
    pub query: String,
    pub applied: bool,
    pub reason: &'static str,
    pub llm_calls: u64,
    pub elapsed_ms: u64,
}

impl RewriteOutcome {
    fn original(query: &str, reason: &'static str, llm_calls: u64, started: Instant) -> Self {
        Self {
            query: query.to_string(),
            applied: false,
            reason,
            llm_calls,
            elapsed_ms: started.elapsed().as_millis() as u64,
        }
    }
}

pub fn rewrite_enabled(raw: Option<&str>) -> bool {
    !matches!(
        raw.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

pub fn rewrite_timeout(raw: Option<&str>) -> Duration {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        return DEFAULT_TIMEOUT;
    };
    match raw.parse::<u64>() {
        Ok(ms) if ms > 0 => Duration::from_millis(ms),
        _ => DEFAULT_TIMEOUT,
    }
}

/// Rewrite only when it is enabled, an LLM is configured, and there is text.
/// An image-only query (blank text) is not rewritten.
pub fn should_rewrite(enabled: bool, query: &str, has_llm: bool) -> bool {
    enabled && has_llm && !query.trim().is_empty()
}

pub fn rewrite_prompt(question: &str) -> String {
    format!(
        "Rewrite this search query for retrieval. Expand abbreviations and add synonyms. \
         Keep it one line. Return the query only, with no explanation.\n\nQuery: {question}"
    )
}

/// Accept a completion only when it cannot poison retrieval.
///
/// Rejects empty text, anything over 400 characters, a second line, a refusal,
/// and a completion that keeps none of the question's significant tokens (the
/// mock provider's `"Mock response"` and a JSON extraction stub both fail here).
pub fn sanitize(question: &str, raw: &str) -> Option<String> {
    let text = raw.trim().trim_matches('"').trim();
    if text.is_empty() || text.chars().count() > MAX_REWRITE_CHARS {
        return None;
    }
    if text.contains('\n') || text.contains('\r') {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("i cannot")
        || lower.contains("i can't")
        || lower.contains("i can’t")
        || lower.contains("as an ai")
        || lower.contains("language model")
    {
        return None;
    }
    if !keeps_a_significant_token(question, text) {
        return None;
    }
    Some(text.to_string())
}

fn keeps_a_significant_token(question: &str, rewrite: &str) -> bool {
    let significant: Vec<String> = tokens(question)
        .filter(|t| t.len() >= 4 || t.chars().any(|c| c.is_ascii_digit()))
        .collect();
    if significant.is_empty() {
        return true;
    }
    let rewritten: std::collections::HashSet<String> = tokens(rewrite).collect();
    significant.iter().any(|t| rewritten.contains(t))
}

fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
}

struct CacheEntry {
    value: String,
    expires: Instant,
}

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, CacheEntry>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_get(question: &str) -> Option<String> {
    let mut guard = cache().lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    guard.retain(|_, entry| entry.expires > now);
    guard.get(question).map(|entry| entry.value.clone())
}

fn cache_put(question: &str, value: &str) {
    let mut guard = cache().lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(
        question.to_string(),
        CacheEntry {
            value: value.to_string(),
            expires: Instant::now() + CACHE_TTL,
        },
    );
}

/// Rewrite `question`. Never returns an error: failure keeps the original text.
pub async fn rewrite_question(
    llm: &dyn LLMProvider,
    question: &str,
    policy: RewritePolicy,
) -> RewriteOutcome {
    let started = Instant::now();
    let question = question.trim();
    if question.is_empty() {
        return RewriteOutcome::original(question, "blank", 0, started);
    }
    if !policy.enabled {
        return RewriteOutcome::original(question, "disabled", 0, started);
    }
    if let Some(cached) = cache_get(question) {
        let applied = cached != question;
        return RewriteOutcome {
            query: cached,
            applied,
            reason: "cache",
            llm_calls: 0,
            elapsed_ms: started.elapsed().as_millis() as u64,
        };
    }
    let prompt = rewrite_prompt(question);
    let completed = tokio::time::timeout(policy.timeout, llm.complete(&prompt)).await;
    let raw = match completed {
        Ok(Ok(response)) => response.content,
        Ok(Err(error)) => {
            tracing::warn!(error = %error, "normal query rewrite failed; using the original");
            return RewriteOutcome::original(question, "error", 1, started);
        }
        Err(_) => {
            tracing::warn!("normal query rewrite timed out; using the original");
            return RewriteOutcome::original(question, "timeout", 1, started);
        }
    };
    let Some(rewritten) = sanitize(question, &raw) else {
        return RewriteOutcome::original(question, "rejected", 1, started);
    };
    cache_put(question, &rewritten);
    if rewritten.eq_ignore_ascii_case(question) {
        return RewriteOutcome::original(question, "unchanged", 1, started);
    }
    RewriteOutcome {
        query: rewritten,
        applied: true,
        reason: "applied",
        llm_calls: 1,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use edgequake_llm::traits::{ChatMessage, CompletionOptions, LLMProvider, LLMResponse};

    use super::*;

    struct Scripted {
        calls: AtomicUsize,
        body: Mutex<Vec<Result<String, ()>>>,
        delay: Duration,
    }

    #[async_trait]
    impl LLMProvider for Scripted {
        fn name(&self) -> &str {
            "scripted"
        }
        fn model(&self) -> &str {
            "scripted"
        }
        fn max_context_length(&self) -> usize {
            1024
        }
        async fn complete(&self, _prompt: &str) -> edgequake_llm::Result<LLMResponse> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            let next = self
                .body
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .pop()
                .unwrap_or(Ok("refunds duration".into()));
            match next {
                Ok(text) => Ok(LLMResponse::new(text, "scripted")),
                Err(()) => Err(edgequake_llm::error::LlmError::ProviderError(
                    "scripted failure".into(),
                )),
            }
        }
        async fn complete_with_options(
            &self,
            prompt: &str,
            _: &CompletionOptions,
        ) -> edgequake_llm::Result<LLMResponse> {
            self.complete(prompt).await
        }
        async fn chat(
            &self,
            _: &[ChatMessage],
            _: Option<&CompletionOptions>,
        ) -> edgequake_llm::Result<LLMResponse> {
            self.complete("").await
        }
    }

    fn policy(enabled: bool, timeout_ms: u64) -> RewritePolicy {
        RewritePolicy {
            enabled,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn scripted(body: Vec<Result<String, ()>>, delay: Duration) -> Arc<Scripted> {
        Arc::new(Scripted {
            calls: AtomicUsize::new(0),
            body: Mutex::new(body.into_iter().rev().collect()),
            delay,
        })
    }

    #[test]
    fn sanitize_rejects_poison() {
        let q = "How long do refunds take?";
        assert!(sanitize(q, "  refunds duration days  ").is_some());
        assert!(sanitize(q, "").is_none());
        assert!(sanitize(q, "line\nline").is_none());
        assert!(sanitize(q, &"x".repeat(401)).is_none());
        assert!(sanitize(q, "I cannot answer that").is_none());
        assert!(sanitize(q, "Mock response").is_none());
        assert!(sanitize(q, r#"{"entities":[]}"#).is_none());
        assert!(sanitize("ERR-4419", "sensor fault").is_none());
        assert!(sanitize("ERR-4419", "ERR-4419 sensor cable").is_some());
    }

    #[test]
    fn a_blank_question_is_not_rewritten() {
        assert!(!should_rewrite(true, "  ", true));
        assert!(!should_rewrite(true, "refunds", false));
        assert!(!should_rewrite(false, "refunds", true));
        assert!(should_rewrite(true, "refunds", true));
    }

    #[test]
    fn env_is_on_unless_explicitly_off() {
        assert!(rewrite_enabled(None));
        assert!(!rewrite_enabled(Some("0")));
        assert!(!rewrite_enabled(Some("false")));
        assert_eq!(rewrite_timeout(None), DEFAULT_TIMEOUT);
        assert_eq!(rewrite_timeout(Some("50")), Duration::from_millis(50));
        assert_eq!(rewrite_timeout(Some("0")), DEFAULT_TIMEOUT);
    }

    #[tokio::test]
    async fn disabled_does_not_call_the_llm() {
        let llm = scripted(vec![], Duration::ZERO);
        let out = rewrite_question(
            llm.as_ref(),
            "How long do refunds take?",
            policy(false, 2000),
        )
        .await;
        assert!(!out.applied);
        assert_eq!(out.reason, "disabled");
        assert_eq!(out.query, "How long do refunds take?");
        assert_eq!(llm.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_error_keeps_the_original() {
        let llm = scripted(vec![Err(())], Duration::ZERO);
        let out = rewrite_question(
            llm.as_ref(),
            "How long do refunds take?",
            policy(true, 2000),
        )
        .await;
        assert!(!out.applied);
        assert_eq!(out.reason, "error");
        assert_eq!(out.query, "How long do refunds take?");
        assert_eq!(out.llm_calls, 1);
    }

    #[tokio::test]
    async fn a_timeout_keeps_the_original() {
        let llm = scripted(
            vec![Ok("refunds duration".into())],
            Duration::from_millis(200),
        );
        let out =
            rewrite_question(llm.as_ref(), "How long do refunds take?", policy(true, 30)).await;
        assert_eq!(out.reason, "timeout");
        assert_eq!(out.query, "How long do refunds take?");
    }

    #[tokio::test]
    async fn a_bad_completion_is_rejected() {
        let llm = scripted(vec![Ok("I cannot help\nwith that".into())], Duration::ZERO);
        let out = rewrite_question(
            llm.as_ref(),
            "How long do refunds take?",
            policy(true, 2000),
        )
        .await;
        assert_eq!(out.reason, "rejected");
        assert!(!out.applied);
    }

    #[tokio::test]
    async fn an_accepted_rewrite_is_what_retrieval_would_use() {
        let llm = scripted(
            vec![Ok("refunds return money duration".into())],
            Duration::ZERO,
        );
        let question = format!("refunds unique {}", uuid::Uuid::new_v4());
        let out = rewrite_question(llm.as_ref(), &question, policy(true, 2000)).await;
        assert!(out.applied, "{out:?}");
        assert_eq!(out.query, "refunds return money duration");
        assert_eq!(out.llm_calls, 1);
        let again = rewrite_question(llm.as_ref(), &question, policy(true, 2000)).await;
        assert_eq!(again.reason, "cache");
        assert_eq!(again.llm_calls, 0);
        assert_eq!(llm.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn the_prompt_asks_for_one_line_and_the_query_only() {
        let prompt = rewrite_prompt("ETA?");
        assert!(prompt.contains("one line"));
        assert!(prompt.contains("ETA?"));
    }
}
