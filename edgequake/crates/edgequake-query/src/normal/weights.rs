//! Fusion weights for the two retrieval legs.
//!
//! SPEC-180 09: the weights come from the environment and a bad value fails startup rather
//! than silently changing ranking. A silent `unwrap_or_default()` is worse than a crash here:
//! the operator believes they tuned recall and the engine is quietly using 1.0 / 1.25.

use std::sync::OnceLock;

/// `EDGEQUAKE_NORMAL_DENSE_WEIGHT` default.
pub const DEFAULT_DENSE_WEIGHT: f32 = 1.0;
/// `EDGEQUAKE_NORMAL_BM25_WEIGHT` default. Lexical evidence is slightly favoured, as
/// exact terms (ids, names, codes) are what dense vectors blur.
pub const DEFAULT_BM25_WEIGHT: f32 = 1.25;

pub const DENSE_ENV: &str = "EDGEQUAKE_NORMAL_DENSE_WEIGHT";
pub const BM25_ENV: &str = "EDGEQUAKE_NORMAL_BM25_WEIGHT";

/// RRF weights of the dense and lexical lists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalWeights {
    pub dense: f32,
    pub bm25: f32,
}

impl Default for NormalWeights {
    fn default() -> Self {
        Self {
            dense: DEFAULT_DENSE_WEIGHT,
            bm25: DEFAULT_BM25_WEIGHT,
        }
    }
}

impl NormalWeights {
    /// Parse the two raw env values. Unset or blank takes the default. A value that is
    /// not a finite number above zero is an error, so a typo fails startup instead of
    /// silently changing ranking.
    pub fn parse(dense: Option<&str>, bm25: Option<&str>) -> Result<Self, String> {
        Ok(Self {
            dense: parse_one(DENSE_ENV, dense, DEFAULT_DENSE_WEIGHT)?,
            bm25: parse_one(BM25_ENV, bm25, DEFAULT_BM25_WEIGHT)?,
        })
    }

    /// Read both weights from the process environment.
    pub fn from_env() -> Result<Self, String> {
        Self::parse(
            std::env::var(DENSE_ENV).ok().as_deref(),
            std::env::var(BM25_ENV).ok().as_deref(),
        )
    }

    /// The weights for this process, or the reason the environment is unusable.
    ///
    /// Resolved once (the environment does not change under a running server) and cached, so
    /// a misconfigured deployment fails on its first normal query instead of scoring with
    /// defaults nobody asked for. The error is kept, not swallowed, so the API can surface it.
    pub fn from_env_cached() -> Result<Self, &'static str> {
        static CACHED: OnceLock<Result<NormalWeights, String>> = OnceLock::new();
        CACHED
            .get_or_init(Self::from_env)
            .as_ref()
            .copied()
            .map_err(|message| message.as_str())
    }
}

fn parse_one(name: &str, raw: Option<&str>, default: f32) -> Result<f32, String> {
    let Some(raw) = raw.map(str::trim).filter(|r| !r.is_empty()) else {
        return Ok(default);
    };
    match raw.parse::<f32>() {
        Ok(v) if v.is_finite() && v > 0.0 => Ok(v),
        _ => Err(format!(
            "{name} must be a finite number greater than 0, got '{raw}'"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_and_blank_use_defaults() {
        assert_eq!(
            NormalWeights::parse(None, None),
            Ok(NormalWeights::default())
        );
        assert_eq!(
            NormalWeights::parse(Some("  "), Some("")),
            Ok(NormalWeights::default())
        );
    }

    #[test]
    fn valid_values_are_taken() {
        let w = NormalWeights::parse(Some("2"), Some(" 0.5 ")).unwrap();
        assert_eq!((w.dense, w.bm25), (2.0, 0.5));
    }

    #[test]
    fn bad_values_name_the_variable() {
        for bad in ["0", "-1", "abc", "NaN", "inf"] {
            let err = NormalWeights::parse(Some(bad), None).unwrap_err();
            assert!(err.contains(DENSE_ENV), "{bad}: {err}");
            let err = NormalWeights::parse(None, Some(bad)).unwrap_err();
            assert!(err.contains(BM25_ENV), "{bad}: {err}");
        }
    }

    #[test]
    fn the_cached_read_reports_a_bad_value_instead_of_defaulting() {
        // A bad value must never come back as the default pair. This is the behaviour the
        // old `NormalWeights::from_env().unwrap_or_default()` got wrong.
        let outcome = NormalWeights::from_env_cached();
        match outcome {
            Ok(w) => {
                assert!(w.dense > 0.0 && w.bm25 > 0.0);
            }
            Err(message) => {
                assert!(message.contains("EDGEQUAKE_NORMAL_"), "{message}");
            }
        }
    }
}
