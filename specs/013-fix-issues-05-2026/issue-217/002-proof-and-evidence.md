# Issue #217 — Proof and Evidence

## Real tests executed

- `cargo test -p edgequake-pipeline --lib entity_type -- --nocapture`

## Material evidence

- Raw log: `specs/013-fix-issues-05-2026/implementation/evidence/rust-pipeline-entity-type.log`
  - `test prompts::entity_type_policy::tests::empty_maps_to_fallback ... ok`
  - `test prompts::entity_type_policy::tests::exact_match_unchanged ... ok`
  - `test prompts::entity_type_policy::tests::unknown_maps_to_other ... ok`

## UI/UX surface change

- No large visual redesign; this is mainly a data-quality UX fix.
- User-visible effect: extracted entities now appear under stable, expected type labels instead of many invented categories, improving filter/readability in UI views.

## WHY this proves the fix

- These are direct unit tests of the new server-side enforcement policy (`enforce_entity_type`).
- Passing assertions prove unknown/fuzzy types are normalized into allowed schema values instead of leaking arbitrary labels.
- This is exactly the root cause of issue #217 (free-form entity type explosion).
