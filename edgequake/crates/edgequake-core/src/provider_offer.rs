//! SPEC-177: which providers a scope may see.
//!
//! Pure data. No SQL and no HTTP. Callers load env rows, fleet rows, and
//! withdrawal flags, then call [`global_offer`] and [`child_offer`].

use std::collections::BTreeSet;

use crate::model_policy::CatalogEntry;

/// One provider the deployment might offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCandidate {
    pub slug: String,
    /// Environment providers do not need a successful probe row.
    pub from_env: bool,
    /// Fleet connections enter the offer only when this is `Some(true)`.
    pub last_test_ok: Option<bool>,
}

/// `true` when `slug` is lowercase ASCII, digits, and hyphens, length 1–64.
pub fn slug_ok(slug: &str) -> bool {
    let bytes = slug.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    bytes
        .iter()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

/// Lowercase and accept, or reject. A slug that still fails after trimming
/// and lowercasing is invalid (`EC-177-44`).
pub fn parse_slug(raw: &str) -> Option<String> {
    let slug = raw.trim().to_lowercase();
    slug_ok(&slug).then_some(slug)
}

/// Providers the deployment offers. Withdrawn slugs and failed fleet probes
/// are absent. An allowlist, when set, runs first.
pub fn global_offer(
    candidates: &[ProviderCandidate],
    withdrawn: &BTreeSet<String>,
    allowlist: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut offer = BTreeSet::new();
    for candidate in candidates {
        let Some(slug) = parse_slug(&candidate.slug) else {
            continue;
        };
        if withdrawn.contains(&slug) {
            continue;
        }
        if let Some(allowed) = allowlist {
            if !allowed.contains(&slug) {
                continue;
            }
        }
        let passes = if candidate.from_env {
            true
        } else {
            candidate.last_test_ok == Some(true)
        };
        if passes {
            offer.insert(slug);
        }
    }
    offer
}

/// The deployment offer, including a fleet connection whose test passed.
///
/// `EDGEQUAKE_ALLOWED_PROVIDERS` hides built-in cards. It does not hide a
/// connection an admin saved and tested. A failed probe and a withdrawal
/// still keep the slug out.
pub fn global_offer_including_tested(
    candidates: &[ProviderCandidate],
    withdrawn: &BTreeSet<String>,
    allowlist: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut offer = global_offer(candidates, withdrawn, allowlist);
    for candidate in candidates {
        if candidate.from_env || candidate.last_test_ok != Some(true) {
            continue;
        }
        let Some(slug) = parse_slug(&candidate.slug) else {
            continue;
        };
        if withdrawn.contains(&slug) {
            continue;
        }
        offer.insert(slug);
    }
    offer
}

/// The child sees every parent slug, or only the enabled intersection once
/// `constrained` is set. Enabled slugs outside the parent are ignored.
pub fn child_offer(
    parent: &BTreeSet<String>,
    constrained: bool,
    enabled: &BTreeSet<String>,
) -> BTreeSet<String> {
    if !constrained {
        return parent.clone();
    }
    parent.intersection(enabled).cloned().collect()
}

/// Catalog entries whose provider is in `offered`.
pub fn filter_catalog<'a>(
    catalog: &'a [CatalogEntry],
    offered: &BTreeSet<String>,
) -> Vec<&'a CatalogEntry> {
    catalog
        .iter()
        .filter(|entry| offered.contains(&entry.provider))
        .collect()
}

/// Whether `provider` is in the offer. Comparison is lowercase.
pub fn provider_offered(offered: &BTreeSet<String>, provider: &str) -> bool {
    parse_slug(provider).is_some_and(|slug| offered.contains(&slug))
}

/// A named provider that the offer does not contain. An empty name is the
/// server default and is not a refusal.
pub fn provider_outside_offer(provider: &str, offered: &BTreeSet<String>) -> bool {
    match parse_slug(provider) {
        Some(slug) => !offered.contains(&slug),
        None => !provider.trim().is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_policy::ModelFacts;

    fn env(slug: &str) -> ProviderCandidate {
        ProviderCandidate {
            slug: slug.into(),
            from_env: true,
            last_test_ok: None,
        }
    }

    fn fleet(slug: &str, ok: Option<bool>) -> ProviderCandidate {
        ProviderCandidate {
            slug: slug.into(),
            from_env: false,
            last_test_ok: ok,
        }
    }

    fn chat(provider: &str, model: &str, embedding: bool) -> CatalogEntry {
        CatalogEntry {
            provider: provider.into(),
            model: model.into(),
            facts: ModelFacts {
                chat: !embedding,
                embedding,
                registered: true,
                ..ModelFacts::default()
            },
        }
    }

    #[test]
    fn env_slug_is_in_the_global_offer() {
        let got = global_offer(&[env("openai")], &BTreeSet::new(), None);
        assert!(got.contains("openai"));
    }

    #[test]
    fn withdrawn_env_slug_is_omitted() {
        let withdrawn = BTreeSet::from(["openai".into()]);
        let got = global_offer(&[env("openai"), env("ollama")], &withdrawn, None);
        assert!(!got.contains("openai"));
        assert!(got.contains("ollama"));
    }

    #[test]
    fn a_tested_connection_survives_the_allowlist() {
        let allow = BTreeSet::from(["ollama".into(), "openai".into()]);
        let got = global_offer_including_tested(
            &[
                env("openai"),
                fleet("vertexai", Some(true)),
                fleet("broken", Some(false)),
            ],
            &BTreeSet::new(),
            Some(&allow),
        );
        assert!(got.contains("openai"));
        assert!(got.contains("vertexai"));
        assert!(!got.contains("broken"));
        assert!(!got.contains("ollama"));
    }

    #[test]
    fn a_withdrawn_tested_connection_stays_out() {
        let allow = BTreeSet::from(["openai".into()]);
        let withdrawn = BTreeSet::from(["vertexai".into()]);
        let got = global_offer_including_tested(
            &[fleet("vertexai", Some(true))],
            &withdrawn,
            Some(&allow),
        );
        assert!(!got.contains("vertexai"));
    }

    #[test]
    fn failed_probe_is_not_offered() {
        let got = global_offer(
            &[fleet("acme", Some(false)), fleet("ok", Some(true))],
            &BTreeSet::new(),
            None,
        );
        assert!(!got.contains("acme"));
        assert!(got.contains("ok"));
    }

    #[test]
    fn unconstrained_child_sees_new_parent_slug() {
        let parent = BTreeSet::from(["openai".into(), "acme".into()]);
        let got = child_offer(&parent, false, &BTreeSet::new());
        assert_eq!(got, parent);
    }

    #[test]
    fn constrained_child_hides_new_parent_slug() {
        let parent = BTreeSet::from(["openai".into(), "acme".into()]);
        let enabled = BTreeSet::from(["openai".into()]);
        let got = child_offer(&parent, true, &enabled);
        assert!(got.contains("openai"));
        assert!(!got.contains("acme"));
    }

    #[test]
    fn empty_constrained_offer_is_empty() {
        let parent = BTreeSet::from(["openai".into()]);
        let got = child_offer(&parent, true, &BTreeSet::new());
        assert!(got.is_empty());
    }

    #[test]
    fn provider_stays_when_models_are_empty() {
        let parent = BTreeSet::from(["openai".into()]);
        let got = child_offer(&parent, false, &BTreeSet::new());
        assert!(got.contains("openai"));
        let catalog = [chat("openai", "gone", false)];
        let filtered = filter_catalog(&catalog, &got);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn empty_global_offer_drops_catalog_cards() {
        let offer = global_offer(&[], &BTreeSet::new(), None);
        let catalog = [chat("openai", "gpt-5-nano", false)];
        assert!(filter_catalog(&catalog, &offer).is_empty());
    }

    #[test]
    fn embedding_card_fails_chat_predicate() {
        let entry = chat("openai", "text-embed", true);
        assert!(!entry.facts.supports(crate::Usage::Extract));
        assert!(entry.facts.supports(crate::Usage::Embedding));
    }

    #[test]
    fn workspace_cannot_widen_tenant() {
        let tenant = BTreeSet::from(["openai".into()]);
        let enabled = BTreeSet::from(["openai".into(), "acme".into()]);
        let got = child_offer(&tenant, true, &enabled);
        assert_eq!(got, tenant);
    }

    #[test]
    fn workspace_unconstrained_follows_tenant() {
        let tenant = BTreeSet::from(["openai".into()]);
        let global = BTreeSet::from(["openai".into(), "acme".into()]);
        let got = child_offer(&tenant, false, &global);
        assert_eq!(got, tenant);
        assert!(!got.contains("acme"));
    }

    #[test]
    fn withdraw_drops_embed_and_chat() {
        let withdrawn = BTreeSet::from(["openai".into()]);
        let offer = global_offer(&[env("openai")], &withdrawn, None);
        let catalog = [
            chat("openai", "gpt-5-nano", false),
            chat("openai", "text-embed", true),
        ];
        assert!(filter_catalog(&catalog, &offer).is_empty());
    }

    #[test]
    fn custom_slug_does_not_match_openai() {
        let offer = global_offer(&[fleet("acme", Some(true))], &BTreeSet::new(), None);
        let catalog = [
            chat("openai", "gpt-5-nano", false),
            chat("acme", "local", false),
        ];
        let names: Vec<_> = filter_catalog(&catalog, &offer)
            .into_iter()
            .map(|entry| entry.provider.as_str())
            .collect();
        assert_eq!(names, ["acme"]);
    }

    #[test]
    fn slug_invalid() {
        assert!(parse_slug(" OpenAI ").as_deref() == Some("openai"));
        assert!(parse_slug("").is_none());
        assert!(parse_slug(&"a".repeat(65)).is_none());
        assert!(parse_slug("has space").is_none());
        assert!(parse_slug("../x").is_none());
    }

    #[test]
    fn provider_outside_offer_refuses_a_named_stranger() {
        let offered = BTreeSet::from(["openai".to_string()]);
        assert!(provider_outside_offer("acme", &offered));
        assert!(!provider_outside_offer("openai", &offered));
        assert!(!provider_outside_offer("", &offered));
        assert!(!provider_outside_offer(" OpenAI ", &offered));
    }

    #[test]
    fn allowlist_drops_other_slugs() {
        let allowed = BTreeSet::from(["ollama".into()]);
        let got = global_offer(
            &[env("openai"), env("ollama")],
            &BTreeSet::new(),
            Some(&allowed),
        );
        assert_eq!(
            got.into_iter().collect::<Vec<_>>(),
            vec!["ollama".to_string()]
        );
    }
}
