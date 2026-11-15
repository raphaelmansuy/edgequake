//! Cluster-wide admission control for local LLM / embedding calls.
//!
//! SPEC-091 QW1 (LAW-Q3, LD-11): the scarce resource is provider inference
//! capacity, shared by every replica. The gate therefore acquires from the
//! **Postgres provider-slot ledger** (`edgequake_tasks::ProviderBudget`) once
//! boot wiring installs it — N replicas cannot multiply provider load by N.
//! The process-local semaphore remains only as the build-time fallback for
//! contexts where no pool exists (unit tests, non-Postgres builds).
//!
//! Env: `EDGEQUAKE_PROVIDER_BUDGET` (default 1; `0` disables) — falls back to
//! legacy `EDGEQUAKE_LOCAL_MAX_INFLIGHT` when unset.

use std::sync::{Arc, LazyLock, Mutex, RwLock};
use std::time::{Duration, Instant};

use edgequake_tasks::{task_lease_ttl_from_env, ProviderSlotGuard, SharedProviderBudget};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Env key for the legacy process-wide local inference permit count.
pub const LOCAL_MAX_INFLIGHT_ENV: &str = edgequake_tasks::LOCAL_MAX_INFLIGHT_ENV;

/// Default in-flight local LLM+embed calls when env is unset.
pub const DEFAULT_LOCAL_MAX_INFLIGHT: usize = 1;

static GATE: LazyLock<LocalInferenceGate> = LazyLock::new(LocalInferenceGate::from_env);

/// Process instance identity for slot attribution (one per boot).
static INSTANCE_ID: LazyLock<String> = LazyLock::new(|| format!("gate-{}", uuid::Uuid::new_v4()));

/// Held admission — drop to release (RAII).
///
/// Both variants are drop-guards; callers simply bind `let _permit = ...`.
pub enum LocalInferencePermit {
    /// Cluster-wide ledger slot (heartbeat + TTL backstop).
    Ledger(ProviderSlotGuard),
    /// Process-local semaphore permit (fallback when no ledger is installed).
    Semaphore(OwnedSemaphorePermit),
}

impl std::fmt::Debug for LocalInferencePermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ledger(g) => f.debug_tuple("Ledger").field(g).finish(),
            Self::Semaphore(_) => f.debug_tuple("Semaphore").finish(),
        }
    }
}

/// Cluster-wide backend installed by boot wiring (state/postgres.rs).
struct LedgerBackend {
    budget: SharedProviderBudget,
    ttl: Duration,
    owner: String,
}

/// Shared admission gate for Ollama / LM Studio traffic.
pub struct LocalInferenceGate {
    /// `None` when disabled (budget = 0).
    semaphore: Option<Arc<Semaphore>>,
    max_inflight: usize,
    /// Last time we logged saturation (rate-limit warn spam).
    last_saturated_log: Mutex<Option<Instant>>,
    /// QW1: cluster-wide provider budget, installed at boot when a pool exists.
    ledger: RwLock<Option<LedgerBackend>>,
}

impl LocalInferenceGate {
    /// Build from env (used once for the process singleton).
    pub fn from_env() -> Self {
        Self::new(usize::from(edgequake_tasks::provider_budget_from_env()))
    }

    /// Construct with an explicit permit count (`0` disables).
    pub fn new(max_inflight: usize) -> Self {
        let semaphore = if max_inflight == 0 {
            None
        } else {
            Some(Arc::new(Semaphore::new(max_inflight)))
        };
        Self {
            semaphore,
            max_inflight,
            last_saturated_log: Mutex::new(None),
            ledger: RwLock::new(None),
        }
    }

    pub fn max_inflight(&self) -> usize {
        self.max_inflight
    }

    pub fn enabled(&self) -> bool {
        self.semaphore.is_some() || self.ledger_installed()
    }

    /// True once boot wiring has installed the cluster-wide budget.
    pub fn ledger_installed(&self) -> bool {
        self.ledger
            .read()
            .expect("local inference gate ledger lock")
            .is_some()
    }

    /// Install the cluster-wide provider budget (boot wiring; LAW-Q3).
    ///
    /// Subsequent local acquires go through the ledger. Installing is
    /// idempotent-safe: a second install replaces the backend (tests).
    pub fn install_provider_budget(&self, budget: SharedProviderBudget) {
        let backend = LedgerBackend {
            budget,
            ttl: task_lease_ttl_from_env(),
            owner: INSTANCE_ID.clone(),
        };
        *self
            .ledger
            .write()
            .expect("local inference gate ledger lock") = Some(backend);
    }

    /// Remove the ledger backend (test teardown).
    #[cfg(test)]
    pub(crate) fn uninstall_provider_budget(&self) {
        *self
            .ledger
            .write()
            .expect("local inference gate ledger lock") = None;
    }

    fn maybe_log_saturated(&self) {
        let mut guard = self
            .last_saturated_log
            .lock()
            .expect("local inference gate log mutex");
        let now = Instant::now();
        let should_log = match *guard {
            None => true,
            Some(prev) => now.duration_since(prev).as_secs() >= 5,
        };
        if should_log {
            *guard = Some(now);
            tracing::warn!(
                max_inflight = self.max_inflight,
                "Local inference gate saturated — waiting for an Ollama/LM Studio slot"
            );
        }
    }

    /// Acquire a permit when the provider is local; no-op otherwise.
    ///
    /// Ledger installed ⇒ cluster path only (LD-11); saturation waits with a
    /// park-not-churn poll (the surrounding call timeout bounds the wait, same
    /// as the semaphore's blocking acquire today).
    pub async fn acquire_for_provider(&self, provider_name: &str) -> Option<LocalInferencePermit> {
        if !edgequake_pipeline::is_local_extraction_provider(provider_name) {
            return None;
        }

        let ledger = self
            .ledger
            .read()
            .expect("local inference gate ledger lock")
            .is_some();
        if ledger {
            return self.acquire_via_ledger(provider_name).await;
        }

        let sem = self.semaphore.clone()?;

        if sem.available_permits() == 0 {
            self.maybe_log_saturated();
        }

        match sem.acquire_owned().await {
            Ok(permit) => Some(LocalInferencePermit::Semaphore(permit)),
            Err(_) => {
                tracing::error!("Local inference gate closed unexpectedly");
                None
            }
        }
    }

    /// Cluster-wide acquire: poll the ledger until a slot frees (same wait
    /// semantics as the semaphore's blocking acquire).
    async fn acquire_via_ledger(&self, provider_name: &str) -> Option<LocalInferencePermit> {
        let provider_key = local_provider_key(provider_name)?;
        loop {
            let (budget, owner, ttl) = {
                let guard = self
                    .ledger
                    .read()
                    .expect("local inference gate ledger lock");
                match guard.as_ref() {
                    Some(backend) => (
                        Arc::clone(&backend.budget),
                        backend.owner.clone(),
                        backend.ttl,
                    ),
                    None => return None, // uninstalled mid-wait (tests)
                }
            };
            match budget.try_acquire(provider_key, &owner, ttl).await {
                Ok(Some(lease)) => {
                    edgequake_observability::metrics::record_provider_slot_acquire(
                        provider_key,
                        "acquired",
                    );
                    return Some(LocalInferencePermit::Ledger(ProviderSlotGuard::start(
                        budget, lease, ttl,
                    )));
                }
                Ok(None) => {
                    edgequake_observability::metrics::record_provider_slot_acquire(
                        provider_key,
                        "busy",
                    );
                    self.maybe_log_saturated();
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                Err(e) => {
                    edgequake_observability::metrics::record_provider_slot_acquire(
                        provider_key,
                        "error",
                    );
                    tracing::warn!(
                        provider = provider_key,
                        error = %e,
                        "Provider budget acquire failed — retrying (call timeout bounds the wait)"
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Normalize local provider names to a ledger key (variants collapse).
fn local_provider_key(provider_name: &str) -> Option<&'static str> {
    match provider_name.trim().to_ascii_lowercase().as_str() {
        "ollama" => Some("ollama"),
        "lmstudio" | "lm-studio" | "lm_studio" => Some("lmstudio"),
        "omlx" | "o-mlx" | "o_mlx" => Some("omlx"),
        "mtplx" | "mtp-lx" | "mtp_lx" => Some("mtplx"),
        "llamacpp" | "llama-server" | "llama.cpp" => Some("llamacpp"),
        "vllm-mlx" | "vllm_mlx" | "vllmmx" => Some("vllm-mlx"),
        "mlx-lm" | "mlx_lm" | "mlxlm" => Some("mlx-lm"),
        "rapid-mlx" | "rapid_mlx" | "rapidmlx" => Some("rapid-mlx"),
        _ => None,
    }
}

/// Process-wide gate singleton.
pub fn global_local_inference_gate() -> &'static LocalInferenceGate {
    &GATE
}

/// Boot wiring (LAW-Q3): install the cluster-wide provider budget on the
/// singleton gate. Called once from `state/postgres.rs` after migrations.
pub fn install_provider_budget(budget: SharedProviderBudget) {
    global_local_inference_gate().install_provider_budget(budget);
}

/// Parse `EDGEQUAKE_LOCAL_MAX_INFLIGHT` (`0` disables; default 1; max 32).
pub fn parse_local_max_inflight(raw: &str) -> usize {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return DEFAULT_LOCAL_MAX_INFLIGHT;
    }
    match trimmed.parse::<usize>() {
        Ok(0) => 0,
        Ok(n) => n.min(32),
        Err(_) => DEFAULT_LOCAL_MAX_INFLIGHT,
    }
}

/// Acquire an admission permit for a local provider call (drop to release).
pub async fn acquire_local_inference_permit(provider_name: &str) -> Option<LocalInferencePermit> {
    // Circuit open → park before taking a scarce slot (releases connection pressure).
    global_local_chat_circuit()
        .wait_until_closed(provider_name)
        .await;
    let before = Instant::now();
    let permit = global_local_inference_gate()
        .acquire_for_provider(provider_name)
        .await;
    if permit.is_some() {
        let wait_ms = before.elapsed().as_millis() as u64;
        if wait_ms > 50 {
            record_local_gate_wait_ms(wait_ms);
            tracing::debug!(
                provider = provider_name,
                gate_wait_ms = wait_ms,
                "Local inference gate wait"
            );
        }
    }
    permit
}

/// Last observed local gate wait (ms) — for Active Run stage messages.
static LAST_GATE_WAIT_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn record_local_gate_wait_ms(ms: u64) {
    LAST_GATE_WAIT_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
    edgequake_observability::metrics::record_local_gate_wait_ms(ms);
}

/// Snapshot the last gate wait for operator UX.
pub fn last_local_gate_wait_ms() -> u64 {
    LAST_GATE_WAIT_MS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Circuit breaker for local chat: open after consecutive network failures.
#[derive(Debug)]
pub struct LocalChatCircuit {
    consecutive_failures: std::sync::atomic::AtomicU32,
    open_until: Mutex<Option<Instant>>,
    failure_threshold: u32,
    open_for: Duration,
}

impl LocalChatCircuit {
    pub fn new(failure_threshold: u32, open_for: Duration) -> Self {
        Self {
            consecutive_failures: std::sync::atomic::AtomicU32::new(0),
            open_until: Mutex::new(None),
            failure_threshold: failure_threshold.max(1),
            open_for,
        }
    }

    pub fn from_env() -> Self {
        let threshold = std::env::var("EDGEQUAKE_LOCAL_CIRCUIT_FAILURES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);
        let open_secs = std::env::var("EDGEQUAKE_LOCAL_CIRCUIT_OPEN_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        Self::new(threshold, Duration::from_secs(open_secs))
    }

    pub fn is_open(&self) -> bool {
        let guard = self.open_until.lock().expect("circuit mutex");
        matches!(*guard, Some(until) if Instant::now() < until)
    }

    pub async fn wait_until_closed(&self, provider_name: &str) {
        if !edgequake_pipeline::is_local_extraction_provider(provider_name) {
            return;
        }
        while self.is_open() {
            tracing::warn!(
                provider = provider_name,
                "Local chat circuit open (provider_unavailable) — pausing new chunk starts"
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub fn record_success(&self) {
        self.consecutive_failures
            .store(0, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut g) = self.open_until.lock() {
            *g = None;
        }
    }

    pub fn record_network_failure(&self) {
        let n = self
            .consecutive_failures
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        edgequake_observability::metrics::record_ollama_network_error();
        if n >= self.failure_threshold {
            if let Ok(mut g) = self.open_until.lock() {
                *g = Some(Instant::now() + self.open_for);
            }
            tracing::error!(
                consecutive = n,
                open_secs = self.open_for.as_secs(),
                "Local chat circuit opened after consecutive network failures"
            );
        }
    }
}

static LOCAL_CHAT_CIRCUIT: LazyLock<LocalChatCircuit> = LazyLock::new(LocalChatCircuit::from_env);

/// Process-wide local chat circuit breaker.
pub fn global_local_chat_circuit() -> &'static LocalChatCircuit {
    &LOCAL_CHAT_CIRCUIT
}

/// Record a successful local chat/complete (closes circuit).
pub fn record_local_chat_success(provider_name: &str) {
    if edgequake_pipeline::is_local_extraction_provider(provider_name) {
        global_local_chat_circuit().record_success();
    }
}

/// Record a local network/transport failure (may open circuit).
pub fn record_local_chat_network_failure(provider_name: &str, error: &str) {
    if !edgequake_pipeline::is_local_extraction_provider(provider_name) {
        return;
    }
    if edgequake_pipeline::is_local_provider_overload_error(error)
        || error.to_ascii_lowercase().contains("timeout")
    {
        global_local_chat_circuit().record_network_failure();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_tasks::MemoryProviderBudget;

    #[test]
    fn parse_defaults_and_disable() {
        assert_eq!(parse_local_max_inflight(""), DEFAULT_LOCAL_MAX_INFLIGHT);
        assert_eq!(parse_local_max_inflight("0"), 0);
        assert_eq!(parse_local_max_inflight("2"), 2);
        assert_eq!(parse_local_max_inflight("99"), 32);
        assert_eq!(parse_local_max_inflight("nope"), DEFAULT_LOCAL_MAX_INFLIGHT);
    }

    #[test]
    fn circuit_opens_after_threshold_network_failures() {
        let c = LocalChatCircuit::new(2, Duration::from_secs(60));
        assert!(!c.is_open());
        c.record_network_failure();
        assert!(!c.is_open());
        c.record_network_failure();
        assert!(c.is_open());
        c.record_success();
        assert!(!c.is_open());
    }

    #[tokio::test]
    async fn gate_limits_concurrent_local_acquires() {
        let gate = LocalInferenceGate::new(1);
        let p1 = gate.acquire_for_provider("ollama").await;
        assert!(p1.is_some());
        assert_eq!(gate.semaphore.as_ref().unwrap().available_permits(), 0);

        // Cloud providers bypass the gate.
        assert!(gate.acquire_for_provider("openai").await.is_none());

        drop(p1);
        let p2 = gate.acquire_for_provider("ollama").await;
        assert!(p2.is_some());
    }

    #[tokio::test]
    async fn gate_disabled_when_zero() {
        let gate = LocalInferenceGate::new(0);
        assert!(!gate.enabled());
        assert!(gate.acquire_for_provider("ollama").await.is_none());
    }

    /// SPEC-091 QW1 / LAW-Q3: with a ledger installed, acquires go through the
    /// cluster budget (semaphore untouched), and release frees the slot.
    #[tokio::test]
    async fn gate_ledger_path_acquires_and_releases() {
        let gate = LocalInferenceGate::new(1);
        let budget: SharedProviderBudget =
            Arc::new(MemoryProviderBudget::new().with_budget("ollama", 1));
        gate.install_provider_budget(budget.clone());
        assert!(gate.ledger_installed());
        // Semaphore stays untouched while the ledger serves.
        assert_eq!(gate.semaphore.as_ref().unwrap().available_permits(), 1);

        let p1 = gate.acquire_for_provider("ollama").await;
        assert!(matches!(p1, Some(LocalInferencePermit::Ledger(_))));

        // Budget exhausted → a direct try_acquire reports saturation (None);
        // the gate's parked acquire frees on drop of p1 (RAII release).
        assert!(budget
            .try_acquire("ollama", "other", Duration::from_secs(60))
            .await
            .unwrap()
            .is_none());
        drop(p1);
        let mut freed = None;
        for _ in 0..100 {
            if let Some(lease) = budget
                .try_acquire("ollama", "other", Duration::from_secs(60))
                .await
                .unwrap()
            {
                freed = Some(lease);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(freed.is_some(), "slot freed on drop within 2s");
        gate.uninstall_provider_budget();
    }

    /// SPEC-091 WP-AC-03: ledger installed ⇒ acquire never returns Semaphore.
    #[tokio::test]
    async fn gate_ledger_never_returns_semaphore_variant() {
        let gate = LocalInferenceGate::new(2);
        let budget: SharedProviderBudget =
            Arc::new(MemoryProviderBudget::new().with_budget("ollama", 2));
        gate.install_provider_budget(budget);
        assert!(gate.ledger_installed());
        for _ in 0..3 {
            let p = gate.acquire_for_provider("ollama").await;
            assert!(
                matches!(p, Some(LocalInferencePermit::Ledger(_))),
                "ledger installed ⇒ never Semaphore permit"
            );
            drop(p);
        }
        assert!(gate.acquire_for_provider("openai").await.is_none());
        gate.uninstall_provider_budget();
    }

    /// Provider-name variants collapse to one ledger key.
    #[test]
    fn provider_key_normalization() {
        assert_eq!(local_provider_key("ollama"), Some("ollama"));
        assert_eq!(local_provider_key(" OLLAMA "), Some("ollama"));
        assert_eq!(local_provider_key("lm-studio"), Some("lmstudio"));
        assert_eq!(local_provider_key("lm_studio"), Some("lmstudio"));
        assert_eq!(local_provider_key("lmstudio"), Some("lmstudio"));
        assert_eq!(local_provider_key("rapid_mlx"), Some("rapid-mlx"));
        assert_eq!(local_provider_key("openai"), None);
    }
}
