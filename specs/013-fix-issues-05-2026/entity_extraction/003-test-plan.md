# Test plan

## Unit — `edgequake-pipeline`

```bash
cargo test -p edgequake-pipeline --lib entity_type -- --nocapture
```

| Test | Asserts |
|------|---------|
| `unknown_maps_to_other` | strict + OTHER in list |
| `unknown_passes_through_when_permissive` | `TELEPHONE_NUMBER` kept |
| `strict_false_no_other_fallback_without_match` | not remapped to OTHER |
| `exact_match_unchanged` | both modes |

## API — `e2e_spec013_github_issues.rs`

`spec013_entity_types_strict_persist_and_defaults`:

1. Create workspace without flag → GET `entity_types_strict == true`
2. PUT `entity_types_strict: false` → persisted
3. PUT `entity_types_strict: true` → key removed, GET true

## Playwright

`e2e/entity-types-strict-limit.spec.ts`:

1. Open dashboard `/workspace`, edit Entity Types
2. Assert checkbox visible and default checked
3. Uncheck, screenshot (unchecked/checked states)
4. Deeplink `/w/[slug]/workspace`: resolve slug, edit, uncheck strict, save, verify API `entity_types_strict: false`

## Proof artifacts

- `implementation/evidence/rust-pipeline-entity-strict.log`
- `implementation/evidence/api-entity-types-strict.log`
- `implementation/004-entity-types-strict-proof.md`
