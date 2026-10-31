//! SPEC-175 W2: model catalog policy (tenant ceiling + workspace narrowing).
//!
//! Pure data and rules. No SQL, no HTTP. The API loads rows and the catalog, then
//! calls [`eligible_models`]. Storage and handlers live outside this crate.
//!
//! - **Ceiling** rows belong to a tenant (`workspace_id` is null).
//! - **Narrowing** rows belong to one workspace.
//! - No ceiling rows for a usage means that usage is unconstrained (no backfill).

mod eligibility;

#[cfg(test)]
mod tests;

pub use eligibility::{
    eligible_models, is_constrained, policy_mode, tenant_policy_mode, usages_left_empty,
    widening_rows,
};

use crate::usage_resolution::Usage;

/// One stored policy row: a provider/model pair for one usage, switched on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyRow {
    pub provider: String,
    pub model: String,
    pub usage: Usage,
    pub enabled: bool,
}

impl PolicyRow {
    /// Whether this row names the same provider/model pair as `entry`.
    pub fn names(&self, entry: &CatalogEntry) -> bool {
        self.provider == entry.provider && self.model == entry.model
    }
}

/// What the catalog knows about one model. Built from the static card and live
/// discovery by the API layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelFacts {
    pub chat: bool,
    pub vision: bool,
    pub embedding: bool,
    /// A static card or user config entry exists. Discovery-only models are not
    /// eligible (LAW-175-11, EC-175-11).
    pub registered: bool,
    /// Latest connection probe for this provider. `Some(false)` removes the model.
    /// `None` (never probed) is not a failure.
    pub probe: Option<bool>,
}

impl ModelFacts {
    /// An embedding card that also embeds images (SPEC-180 08). For embedding
    /// cards the catalog's `supports_vision` flag carries this meaning.
    pub fn embeds_image(&self) -> bool {
        self.embedding && self.vision
    }

    /// Capability predicate: can this model serve `usage` at all?
    pub fn supports(&self, usage: Usage) -> bool {
        match usage {
            Usage::Embedding => self.embedding,
            Usage::Vlm => self.chat && self.vision,
            Usage::Rerank => false,
            Usage::Extract | Usage::Query | Usage::Summary | Usage::Keyword | Usage::Decision => {
                self.chat
            }
        }
    }
}

/// One deployment catalog entry as seen by policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub provider: String,
    pub model: String,
    pub facts: ModelFacts,
}

/// Whether a usage is constrained by the tenant ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyMode {
    Unconstrained,
    Constrained,
}

impl PolicyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconstrained => "unconstrained",
            Self::Constrained => "constrained",
        }
    }
}
