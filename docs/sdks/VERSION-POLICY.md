---
title: SDK version policy
description: How EdgeQuake SDK package versions relate to the server product version, and what to pin when you install a client.
---

# SDK version policy

This page explains why the server is **0.32.2** while the clients say **0.4.0**, and how to choose a pin. It is for release managers and integrators.

## Two version lines

| Surface | Version today | Meaning |
|---------|---------------|---------|
| Server / Docker / workspace crates | **0.32.2** | Product release (`ghcr.io/raphaelmansuy/edgequake:0.32.2`) |
| SDK source trees under `sdks/` | **0.4.0** | Client library semver |
| Published registries | May lag source | PyPI `edgequake-sdk` **0.3.0**, npm `edgequake-sdk` **0.1.0**, crates.io `edgequake-sdk` **0.4.0**. Others unpublished. |

A `0.4.x` client talking to a `0.32.x` server is intentional when the OpenAPI paths match. SDK majors bump when the **client API** breaks, not on every server patch.

## Rules

1. **Contract source of truth** is the server OpenAPI (`routes.rs` → snapshot → codegen).
2. SDK package versions track **client surface** changes, not every server patch.
3. When an SDK breaking change ships, bump the SDK major and document the minimum server version in that SDK README.
4. Root GitHub Actions (`.github/workflows/sdk-*.yml`) own CI. Nested `sdks/*/.github/workflows` are ignored by GitHub Actions.
5. New server routes (for example v0.33.0 Connections) may ship before any SDK wraps them. Call raw HTTP until a client release adds them.

## What to pin

| Goal | Pin |
|------|-----|
| Match published PyPI | `edgequake-sdk==0.3.0` |
| Match published npm | `edgequake-sdk@0.1.0` |
| Match published crates.io | `edgequake-sdk = "0.4"` |
| Match monorepo HEAD | Install from `sdks/<lang>` path (source 0.4.0) |
| Match a server image | Pin the **server** tag (`0.32.2`) separately from the SDK |

Related: [SDK overview](README.md), [Brutal assessment](BRUTAL-ASSESSMENT.md).
