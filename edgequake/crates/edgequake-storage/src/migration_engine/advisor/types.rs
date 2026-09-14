//! SPEC-091 Migration Console — derived posture types.
//!
//! LAW-C1 (derive, never store): every type here is computed from the live
//! schema on each call; nothing is persisted. The rule engine in `rules.rs` is
//! a pure function over these types, so guidance is unit-testable without a
//! database.

use serde::Serialize;

use crate::migration_engine::MigrationMode;

/// Fact #3 — per-family write/read mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FamilyMode {
    Kv,
    Dual,
    Relational,
}

impl FamilyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kv => "kv",
            Self::Dual => "dual",
            Self::Relational => "relational",
        }
    }

    /// Whether this mode still writes the generic KV store (stale-flag risk).
    pub fn writes_kv(self) -> bool {
        matches!(self, Self::Kv | Self::Dual)
    }
}

/// The derived lifecycle phase of one KV family (state machine, doc 15 §4).
/// Declaration order is the cutover order — `Ord` lets the aggregate pick the
/// bottleneck (earliest) family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyPhase {
    /// mode=Kv/Dual with un-migrated durable rows and no active backfill.
    NotStarted,
    /// chunk only: dual-writing, backfill not yet complete.
    DualWriting,
    /// engine job moving historical rows (pending/preflight/running/paused).
    Backfilling,
    /// engine job in the verify step.
    Verifying,
    /// durable data fully in the typed SSOT (+ chunk verify clean); flip the flag.
    ReadyToFlip,
    /// mode=Relational, KV store still present (soaking before the drop).
    Flipped,
    /// all families relational and total durable residue = 0; safe to drop.
    ReadyToDrop,
    /// KV relations gone (terminal).
    Dropped,
}

impl FamilyPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "NotStarted",
            Self::DualWriting => "DualWriting",
            Self::Backfilling => "Backfilling",
            Self::Verifying => "Verifying",
            Self::ReadyToFlip => "ReadyToFlip",
            Self::Flipped => "Flipped",
            Self::ReadyToDrop => "ReadyToDrop",
            Self::Dropped => "Dropped",
        }
    }
}

/// Aggregate "where am I" rollup across all families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CutoverPhase {
    NotStarted,
    InProgress,
    ReadyToDrop,
    Dropped,
}

impl CutoverPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "NOT STARTED",
            Self::InProgress => "IN PROGRESS",
            Self::ReadyToDrop => "READY TO DROP",
            Self::Dropped => "DROPPED",
        }
    }
}

/// Fact #1 — a snapshot of an engine job (chunk backfill today).
#[derive(Debug, Clone, Serialize)]
pub struct JobSnapshot {
    pub step_id: String,
    pub job_id: Option<String>,
    pub state: String,
    pub completion_pct: Option<f64>,
    pub processed_count: i64,
    pub estimated_total: Option<i64>,
    pub rows_per_sec: Option<f64>,
    pub eta_seconds: Option<f64>,
    pub throttle_reason: Option<String>,
    pub last_error: Option<String>,
}

impl JobSnapshot {
    /// States in which the job is actively migrating (or interruptibly parked).
    pub fn is_active(&self) -> bool {
        matches!(
            self.state.as_str(),
            "pending" | "preflight" | "running" | "paused"
        )
    }
}

/// Fact #4 — verification summary (chunk content equality).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct VerifySummary {
    pub expected: i64,
    pub actual: i64,
    pub sampled: usize,
    pub mismatches: usize,
}

impl VerifySummary {
    /// Same pass rule as `runner::VerifyReport::passes` (SSOT).
    pub fn passes(&self) -> bool {
        super::super::runner::VerifyReport {
            metric: String::new(),
            expected: self.expected,
            actual: self.actual,
            sampled: self.sampled,
            mismatches: self.mismatches,
        }
        .passes()
    }
}

/// Fact #5 — durable KV residue per category. Mirrors the migration-125 drop
/// guard predicates exactly (LAW-C3): a row counts only when it is durable AND
/// not yet represented in its typed SSOT. Transient families (checkpoints,
/// caches) are excluded by design.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ResidueReport {
    pub chunk_text: i64,
    pub doc_shells: i64,
    pub lineage: i64,
    pub multimodal: i64,
    pub doc_hash: i64,
    pub staging_hash: i64,
    pub wsdoc: i64,
    pub injection: i64,
}

impl ResidueReport {
    pub fn total(&self) -> i64 {
        self.chunk_text
            + self.doc_shells
            + self.lineage
            + self.multimodal
            + self.doc_hash
            + self.staging_hash
            + self.wsdoc
            + self.injection
    }

    /// Accumulate another table's residue (used to sum across `eq_*_kv` tables).
    pub fn add(&mut self, other: &Self) {
        self.chunk_text += other.chunk_text;
        self.doc_shells += other.doc_shells;
        self.lineage += other.lineage;
        self.multimodal += other.multimodal;
        self.doc_hash += other.doc_hash;
        self.staging_hash += other.staging_hash;
        self.wsdoc += other.wsdoc;
        self.injection += other.injection;
    }

    /// Residue attributable to one family (None for transient families, which
    /// the drop guard ignores).
    pub fn for_family(&self, family: &str) -> i64 {
        match family {
            "CHUNK" => self.chunk_text,
            "METADATA" => self.doc_shells,
            "WSDOC" => self.wsdoc,
            "STAGING_HASH" => self.staging_hash,
            "DOC_HASH" => self.doc_hash,
            "ARTIFACT" => self.lineage + self.multimodal,
            "INJECTION" => self.injection,
            _ => 0,
        }
    }

    /// Human-readable non-zero breakdown, e.g. for the drop-blocked evidence.
    pub fn breakdown(&self) -> String {
        let parts = [
            ("chunk_text", self.chunk_text),
            ("doc_shells", self.doc_shells),
            ("lineage", self.lineage),
            ("multimodal", self.multimodal),
            ("doc_hash", self.doc_hash),
            ("staging_hash", self.staging_hash),
            ("wsdoc", self.wsdoc),
            ("injection", self.injection),
        ];
        let listed: Vec<String> = parts
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(k, n)| format!("{k}={n}"))
            .collect();
        if listed.is_empty() {
            "none".to_string()
        } else {
            listed.join(", ")
        }
    }
}

/// Static descriptor for a KV family (OCP: add a family by adding a row here,
/// not by editing control flow).
#[derive(Debug, Clone, Copy)]
pub struct FamilySpec {
    pub name: &'static str,
    /// The chunk-text family uses the 3-mode authority flag and the engine
    /// backfill + content verify; all others use the 2-mode family flag.
    pub is_chunk: bool,
    /// Durable families block the drop (migration-125 guard); transient ones
    /// (checkpoints, caches, quarantine) are excluded by design.
    pub durable: bool,
    /// Typed SSOT table(s) counted for fact #6.
    pub typed_tables: &'static [&'static str],
    /// The exact env var that owns this family's mode (SSOT for guidance).
    pub env_flag: &'static str,
}

/// The full durable + transient family set (doc 15 §3).
pub const FAMILIES: &[FamilySpec] = &[
    FamilySpec {
        name: "CHUNK",
        is_chunk: true,
        durable: true,
        typed_tables: &["chunks"],
        env_flag: "EDGEQUAKE_CHUNK_TEXT_AUTHORITY",
    },
    FamilySpec {
        name: "METADATA",
        is_chunk: false,
        durable: true,
        typed_tables: &["documents"],
        env_flag: "EDGEQUAKE_KV_FAMILY_METADATA",
    },
    FamilySpec {
        name: "WSDOC",
        is_chunk: false,
        durable: true,
        typed_tables: &["documents"],
        env_flag: "EDGEQUAKE_KV_FAMILY_WSDOC",
    },
    // `staging:hash:` residue rolls into DOC_HASH at runtime (no separate flag).
    FamilySpec {
        name: "DOC_HASH",
        is_chunk: false,
        durable: true,
        typed_tables: &["ingestion_dedup"],
        env_flag: "EDGEQUAKE_KV_FAMILY_DOC_HASH",
    },
    FamilySpec {
        name: "COMPENSATION_QUARANTINE",
        is_chunk: false,
        durable: false,
        typed_tables: &["compensation_quarantine"],
        env_flag: "EDGEQUAKE_KV_FAMILY_COMPENSATION_QUARANTINE",
    },
    FamilySpec {
        name: "CHECKPOINT",
        is_chunk: false,
        durable: false,
        typed_tables: &["pipeline_checkpoints"],
        env_flag: "EDGEQUAKE_KV_FAMILY_CHECKPOINT",
    },
    FamilySpec {
        name: "ARTIFACT",
        is_chunk: false,
        durable: true,
        typed_tables: &["document_artifacts"],
        env_flag: "EDGEQUAKE_KV_FAMILY_ARTIFACT",
    },
    FamilySpec {
        name: "INJECTION",
        is_chunk: false,
        durable: true,
        typed_tables: &["documents"],
        env_flag: "EDGEQUAKE_KV_FAMILY_INJECTION",
    },
    FamilySpec {
        name: "CACHE",
        is_chunk: false,
        durable: false,
        typed_tables: &["llm_cache"],
        env_flag: "EDGEQUAKE_KV_FAMILY_CACHE",
    },
];

/// Per-family derived posture (facts + derived phase).
#[derive(Debug, Clone, Serialize)]
pub struct FamilyPosture {
    pub family: &'static str,
    pub mode: FamilyMode,
    pub phase: FamilyPhase,
    pub durable: bool,
    pub backfill: Option<JobSnapshot>,
    pub verify: Option<VerifySummary>,
    pub kv_residue_rows: i64,
    pub typed_rows: i64,
    pub typed_tables: &'static [&'static str],
    pub env_flag: &'static str,
}

/// SPEC-091 W3 — derived VECTOR posture (chunk embeddings cutover). Separate
/// from the KV families: vectors live in `eq_*_vectors`, not the KV store, so
/// they are tracked on their own row (doc 15 §3 extension for W3).
#[derive(Debug, Clone, Serialize)]
pub struct VectorPosture {
    /// Backend flag (`legacy_tables` | `chunk_embeddings`).
    pub backend: String,
    /// Engine W3 job snapshot (None when ledger absent / never registered).
    pub backfill: Option<JobSnapshot>,
    /// Engine IW2 fleet job snapshot.
    pub fleet_backfill: Option<JobSnapshot>,
    /// Chunk-embedding verify only (migration 126 gate).
    pub verify_chunk: Option<VerifySummary>,
    /// Fleet-embedding verify only (migration 131 gate; provenance coverage).
    pub verify_fleet: Option<VerifySummary>,
    /// Combined chunk+fleet verify (JSON compat / display aggregate).
    pub verify: Option<VerifySummary>,
    /// Dual-legacy→one-typed collision stalls (entity/rel alias residue).
    pub provenance_stall_rows: i64,
    /// Typed `chunk_embeddings` row count.
    pub typed_rows: i64,
    /// Typed fleet row counts (IW2).
    pub typed_entity_rows: i64,
    pub typed_relationship_rows: i64,
    pub typed_report_rows: i64,
    /// Legacy chunk rows remaining in `eq_*_vectors`.
    pub legacy_chunk_rows: i64,
    /// Legacy entity/relationship/report rows remaining in `eq_*_vectors`.
    pub legacy_fleet_rows: i64,
    /// Legacy chunk rows without typed coverage (SPEC-111 — drop readiness).
    /// Equals `uncovered_chunk_missing_spine_rows + uncovered_chunk_missing_embedding_rows`
    /// (≡ DROP 126). Split is advisor honesty only — does not weaken the gate.
    pub uncovered_chunk_rows: i64,
    /// Uncovered because `public.chunks` spine is missing (SPEC-396).
    pub uncovered_chunk_missing_spine_rows: i64,
    /// Uncovered because spine exists but `chunk_embeddings` does not (SPEC-396).
    pub uncovered_chunk_missing_embedding_rows: i64,
    /// Legacy fleet rows without typed coverage (SPEC-111).
    pub uncovered_fleet_rows: i64,
    /// Migration 126 applied — chunk-dedicated legacy vector fleet retired.
    pub chunk_fleet_dropped: bool,
    /// All legacy `eq_*_vectors` relations dropped (terminal, IW2+).
    pub dropped: bool,
}

impl VectorPosture {
    /// Readiness to flip `EDGEQUAKE_VECTOR_BACKEND=chunk_embeddings`: typed
    /// coverage is complete and the sampled verify passes.
    pub fn ready_to_flip(&self) -> bool {
        !self.dropped
            && self.legacy_chunk_rows > 0
            && self.uncovered_chunk_rows == 0
            && self.verify_chunk.map(|v| v.passes()).unwrap_or(false)
    }

    /// Readiness to **retire** the legacy chunk-vector fleet (drop
    /// `eq_*_vectors` chunk relations). Stricter than `ready_to_flip`: the
    /// read backend must already be authoritative on typed `chunk_embeddings`,
    /// every legacy chunk row must be **covered** in typed SSOT
    /// (`uncovered_chunk_rows == 0` — LAW-111-2; emptiness is a post-condition
    /// of DROP), and chunk verify must pass. Terminal once `dropped` is true.
    pub fn retirable(&self) -> bool {
        self.chunk_retirable()
    }

    /// Readiness for migration 126 (chunk-only legacy drop).
    pub fn chunk_retirable(&self) -> bool {
        !self.dropped
            && self.backend_reads_typed()
            && self.uncovered_chunk_rows == 0
            && self.verify_chunk.map(|v| v.passes()).unwrap_or(false)
    }

    /// Readiness for migration 131 (full legacy vector fleet drop).
    pub fn fleet_retirable(&self) -> bool {
        !self.dropped
            && self.backend_reads_typed()
            && self.uncovered_chunk_rows == 0
            && self.uncovered_fleet_rows == 0
            && self.verify_fleet.map(|v| v.passes()).unwrap_or(false)
    }

    /// Backend already reads typed SSOT (chunk or full fleet).
    pub fn backend_reads_typed(&self) -> bool {
        matches!(
            self.backend.as_str(),
            "typed_embeddings" | "chunk_embeddings"
        )
    }
}

/// The whole derived posture (LAW-C1). Recomputed from the schema per call.
#[derive(Debug, Clone, Serialize)]
pub struct MigrationPosture {
    /// Fact #2 — migration 125 applied OR no `eq_*_kv` relations remain.
    pub kv_store_dropped: bool,
    /// EC-C3 — the migration engine ledger (migration 106) is installed.
    pub engine_installed: bool,
    /// Fact #8.
    pub engine_mode: MigrationMode,
    /// Fact #7.
    pub serving_fence_on: bool,
    pub families: Vec<FamilyPosture>,
    /// Fact #5 — total durable residue across every remaining `eq_*_kv` table.
    pub residue: ResidueReport,
    pub cutover_phase: CutoverPhase,
    /// SPEC-091 W3 — chunk-embedding VECTOR posture.
    pub vector: VectorPosture,
}

impl MigrationPosture {
    /// Global ready-to-drop signal (stricter than the 125 guard: also requires
    /// every family's flag off KV so no post-drop write can 42P01).
    pub fn global_ready_to_drop(&self) -> bool {
        !self.kv_store_dropped
            && self.residue.total() == 0
            && self
                .families
                .iter()
                .all(|f| f.mode == FamilyMode::Relational)
    }

    pub fn family(&self, name: &str) -> Option<&FamilyPosture> {
        self.families.iter().find(|f| f.family == name)
    }
}

/// The kind of an operator instruction (doc 15 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstrKind {
    Wait,
    Action,
    Confirm,
    Done,
    Blocked,
}

impl InstrKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wait => "WAIT",
            Self::Action => "ACTION",
            Self::Confirm => "CONFIRM",
            Self::Done => "DONE",
            Self::Blocked => "BLOCKED",
        }
    }
}

/// One explicit, ordered operator instruction (LAW-C6: a sentence per metric).
#[derive(Debug, Clone, Serialize)]
pub struct Instruction {
    pub ordinal: u32,
    pub kind: InstrKind,
    pub summary: String,
    /// Exact copy-pasteable flag/command, when one exists.
    pub command: Option<String>,
    /// The gate that must be green first, when relevant.
    pub gate: Option<String>,
    /// The numbers behind the instruction (pct, mismatches, residue).
    pub evidence: String,
}

/// A mutation the console offers, re-checked live at execution (LAW-C4/C5).
#[derive(Debug, Clone, Serialize)]
pub struct GuardedAction {
    pub verb: String,
    pub target: String,
    pub enabled: bool,
    /// Why the action is disabled (operator-readable), when it is.
    pub gate_reason: Option<String>,
    pub requires_confirmation: bool,
    pub irreversible: bool,
}

impl GuardedAction {
    pub fn enabled(verb: &str, target: &str) -> Self {
        Self {
            verb: verb.to_string(),
            target: target.to_string(),
            enabled: true,
            gate_reason: None,
            requires_confirmation: false,
            irreversible: false,
        }
    }

    pub fn gated(verb: &str, target: &str, reason: String) -> Self {
        Self {
            verb: verb.to_string(),
            target: target.to_string(),
            enabled: false,
            gate_reason: Some(reason),
            requires_confirmation: false,
            irreversible: false,
        }
    }

    pub fn confirm(mut self, irreversible: bool) -> Self {
        self.requires_confirmation = true;
        self.irreversible = irreversible;
        self
    }
}

/// The output of the pure rule engine: ordered instructions + gated actions.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Guidance {
    pub instructions: Vec<Instruction>,
    pub actions: Vec<GuardedAction>,
}
