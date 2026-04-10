# Orient

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Repository HEAD under analysis: `27f403c06b340651b7497e1e36873837ad1415ed`

## Analysis

Duplicated test setup is low-level friction that multiplies maintenance cost every time provider configuration evolves. In this repository, provider routing behavior is a fast-moving area, so repeated helper code is more than cosmetic duplication: it raises the probability of tests drifting apart and accidentally covering different setup paths.

## Options Considered

### Option A: Leave helpers duplicated and only keep the recent style cleanups

- Benefit: zero refactor risk.
- Risk: keeps seven-plus local helper variants alive, making future provider changes noisy and error-prone.
- Rejected.

### Option B: Introduce one overly generic builder that accepts a huge matrix of optional fields

- Benefit: maximum reuse.
- Risk: hides test intent behind a complicated helper API and increases accidental misuse.
- Rejected.

### Option C: Add a small set of focused helpers to `tests/common/mod.rs` and update the touched files to use them

- Benefit: DRY improvement with limited API surface.
- Benefit: aligns with Rust module guidance to group related code behind a stable module boundary.
- Benefit: preserves test readability because each call still spells out provider intent.
- Accepted.

## Risk Notes

- Integration tests are separate crates, so each file that uses the shared helpers must opt into `mod common;` explicitly.
- Using the broader `clear_provider_detection_env()` helper in place of narrower local cleanup reduces machine-specific flakiness, which is a desired tightening rather than a behavior regression.
