---
title: "SDK version policy (SPEC-083 X-33)"
---

# SDK version policy (SPEC-083 X-33)

## Product vs package semver

| Surface | Version today | Meaning |
|---------|---------------|---------|
| Server / webui / workspace crates | **0.20.x** | Product release (GHCR / `Cargo.toml` workspace) |
| Hand-written SDKs (Rust/Python/TS/…) | **0.4.x** (aligned) | Client library semver — **not** the product version |

SDK major/minor may lag the product version when the HTTP contract remains compatible. A `0.4.x` client talking to a `0.20.x` server is intentional when OpenAPI paths match.

## Policy

1. **Contract SSOT** is server OpenAPI (`routes.rs` → snapshot → codegen).
2. SDK package versions track **client API surface** changes, not every server patch.
3. When an SDK breaking change ships, bump SDK major; document server min version in the SDK README.
4. Root GitHub Actions (`.github/workflows/sdk-*.yml`) own CI — nested `sdks/*/.github/workflows` are ignored by GHA (D-48).

## Contract

`contract_sdk_major_matches_server_policy` asserts Tier-1 SDK versions are documented here and present in package manifests (currently `0.4.0` client track).
