---
title: "Upgrade to EdgeQuake v0.40.13"
description: "Upgrade from v0.40.12 to v0.40.13. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.13

> **From:** v0.40.12 (schema **231**) · **To:** v0.40.13 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

## Read this first: what v0.40.13 repairs

v0.40.8 through v0.40.12 were tagged from a side branch and were **never merged back** to `edgequake-main`. That left the mainline at schema **227** while the published product served **231**. A release cut from the mainline would have declared `compat_serve_max = 227` against databases already at 231 — a downgrade, not an upgrade.

v0.40.13 merges that line home. It is the first release from the mainline since v0.40.7 that can serve schema 231. **If you run v0.40.7 or earlier, follow [upgrade-to-0.40.8.md](upgrade-to-0.40.8.md) for migrations 228–231 first**; this guide assumes you are already at 231.

## Order of operations

```bash
# 1. Back up. This is a routine schema-231 release, but it is the first
#    mainline build that carries 228–231 together with the v0.40.13 fixes.
# 2. Roll the API replicas to 0.40.13. `edgequake migrate` reports no-op at 231.
# 3. Roll the WebUI to 0.40.13.
```

## What changes for an operator

**Vertex picture search no longer needs `GOOGLE_CLOUD_PROJECT` when a project is saved on the connection.** A project stored on the connection wins over the `project_id` inside the service-account file. This was introduced on the v0.40.12 line and is now on the mainline.

**An API key still cannot embed pictures on Vertex.** Vertex accepts OAuth2 only. A workspace whose Vertex connection holds an API key keeps text search and loses picture search, and the server logs one line per process explaining it. Save a service-account JSON, or leave the secret blank to use the server's Google identity.

**Editing a connection keeps the stored key.** Clearing the key field on an existing connection leaves the stored credential in place; a new connection still requires a key. Added in v0.40.13.

**A chat answer that contains many unclosed HTML tags no longer freezes the answer pane.** The inline-markup pass was quadratic in the number of unclosed tags: an answer with 32 000 of them took 5.5 seconds per render. It is now linear (0.4 ms for the same input), with byte-identical output.

**Scrolling a large document no longer re-scans it per visible row.** The citation-highlight mapper walked the whole document once per rendered row. It now resolves the highlight offsets once per render. On a 2 MB document this was 11.7 ms per frame and is now 1.5 ms.

**A release can no longer ship a schema behind a published one.** `make release-gates` and CI now fail if `compat_serve_max` is below any published `vX.Y.Z` tag. This is the check that was missing between v0.40.7 and v0.40.12.

## Verify after the roll

```bash
curl -s http://localhost:8080/health | jq '{version, storage_mode}'
curl -s http://localhost:8080/api-docs/openapi.json | jq -r '.info.version'   # 0.40.13
```

**SPEC-001 Acc** is not re-scored on this cut. Mix retrieval is unchanged. The pack `medical-mid-20261024T031725Z` stays the current result: EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]**. The interval includes 0, so this is a statistical tie, not a win.
