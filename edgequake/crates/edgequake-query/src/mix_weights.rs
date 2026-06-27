//! Mix-mode weight resolution (SPEC-022 P-H6 / DRY SSOT).
//!
//! Weights live on `QueryEngineConfig` by default; HTTP/SDK may override per request.

use serde::{Deserialize, Serialize};

use crate::engine_impl::QueryEngineConfig;

/// Optional per-request Mix weight override (unset fields use engine config defaults).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MixWeightOverride {
    #[serde(default)]
    pub local: Option<f32>,
    #[serde(default)]
    pub global: Option<f32>,
    #[serde(default)]
    pub naive: Option<f32>,
}

impl MixWeightOverride {
    pub fn is_set(&self) -> bool {
        self.local.is_some() || self.global.is_some() || self.naive.is_some()
    }
}

/// Normalize Mix weights to sum to 1 (P-G8 E24/E25).
pub fn normalized_mix_weights(
    config: &QueryEngineConfig,
    override_weights: Option<&MixWeightOverride>,
) -> (f32, f32, f32) {
    let l = override_weights
        .and_then(|o| o.local)
        .unwrap_or(config.mix_local_weight);
    let g = override_weights
        .and_then(|o| o.global)
        .unwrap_or(config.mix_global_weight);
    let n = override_weights
        .and_then(|o| o.naive)
        .unwrap_or(config.mix_naive_weight);
    let sum = l + g + n;
    if !sum.is_finite() || sum <= 0.0 {
        tracing::warn!(
            mix_local_weight = l,
            mix_global_weight = g,
            mix_naive_weight = n,
            "Mix weights sum to 0 or are non-finite; falling back to equal weights (P-G8 E24)"
        );
        (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0)
    } else {
        (l / sum, g / sum, n / sum)
    }
}
