//! Vision → EdgeParse fallback policy (SPEC-017 P1-09).
//!
//! Centralizes when a vision backend failure should degrade to EdgeParse
//! instead of failing the ingestion task outright.

use crate::PdfParserBackend;

/// Classification of vision extraction failures for fallback decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionFailureKind {
    Timeout,
    ProviderUnavailable,
    ConversionFailed,
    FeatureUnavailable,
}

impl VisionFailureKind {
    pub fn as_detail_str(self) -> &'static str {
        match self {
            Self::Timeout => "timed out",
            Self::ProviderUnavailable => "provider unavailable",
            Self::ConversionFailed => "conversion failed",
            Self::FeatureUnavailable => "vision feature unavailable",
        }
    }
}

/// Returns true when a vision backend request should fall back to EdgeParse.
pub fn should_fallback_to_edgeparse(
    requested_backend: PdfParserBackend,
    failure: VisionFailureKind,
) -> bool {
    if requested_backend != PdfParserBackend::Vision {
        return false;
    }

    matches!(
        failure,
        VisionFailureKind::Timeout
            | VisionFailureKind::ProviderUnavailable
            | VisionFailureKind::ConversionFailed
            | VisionFailureKind::FeatureUnavailable
    )
}

/// User-visible notice when vision extraction degrades to EdgeParse.
pub fn build_edgeparse_fallback_message(provider: &str, detail: &str) -> String {
    format!(
        "Vision extraction via {provider} was unavailable ({detail}). Falling back to EdgeParse for a more reliable text extraction."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_failures_trigger_edgeparse_fallback() {
        for failure in [
            VisionFailureKind::Timeout,
            VisionFailureKind::ProviderUnavailable,
            VisionFailureKind::ConversionFailed,
            VisionFailureKind::FeatureUnavailable,
        ] {
            assert!(should_fallback_to_edgeparse(
                PdfParserBackend::Vision,
                failure
            ));
        }
    }

    #[test]
    fn edgeparse_requests_do_not_self_fallback() {
        assert!(!should_fallback_to_edgeparse(
            PdfParserBackend::EdgeParse,
            VisionFailureKind::Timeout
        ));
    }

    #[test]
    fn fallback_message_includes_provider_and_detail() {
        let msg = build_edgeparse_fallback_message("ollama", "timed out");
        assert!(msg.contains("ollama"));
        assert!(msg.contains("timed out"));
        assert!(msg.contains("EdgeParse"));
    }
}
