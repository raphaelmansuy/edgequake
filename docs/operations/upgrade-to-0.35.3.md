---
title: "Upgrade to EdgeQuake v0.35.3"
description: "Upgrade from v0.35.2 to v0.35.3. Schema stays 207."
---

# Upgrade to EdgeQuake v0.35.3

> **From:** v0.35.2 (schema **207**) · **To:** v0.35.3 (schema **207**)

No new migration. `edgequake migrate` on a database already at 207 is a no-op. From v0.35.0, apply [upgrade-to-0.35.1.md](upgrade-to-0.35.1.md) first. Apache AGE is still dropped only when you pass `--drop-age`.

`release_for_schema` still maps 207 to v0.35.1, the release that first shipped that schema. v0.35.3 serves the same schema.

**demo.edgequake.com:** this cut installs the 0.35.3 API and WebUI. Password auth remains the demo default.

## What changed

- A question that names a stored entity expands that entity's edges and moves its source pages to the front of the prompt. Abbreviation pages stay ahead of those pages. The front pages are the best lexical matches among that entity's own source pages.
- A keyword that is stored with underscores is also looked up in that form, so a spaced phrase can hit the graph label.

## Operator steps

```bash
pg_dump -Fc "$DATABASE_URL" > edgequake-$(date +%F).dump
edgequake migrate status
```

A database at 207 prints `latest=207`. Full command reference: [upgrading.md](upgrading.md).
