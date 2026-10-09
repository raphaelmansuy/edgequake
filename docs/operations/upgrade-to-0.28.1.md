---
title: "Upgrade to EdgeQuake v0.28.1"
description: "Per-release upgrade notes for EdgeQuake v0.28.1: what changed and what to run."
---

# Upgrade to EdgeQuake v0.28.1

> **From:** v0.28.0 · **To:** v0.28.1 · **CD:** GHCR (`edgequake`,
> `edgequake-frontend`, `edgequake-postgres`)

Patch cut: CI/docs/compose hygiene after v0.28.0. **Schema stays at 160** —
no new migration. If you already migrated to 0.28.0, you do not need
`edgequake migrate` for this bump.

## Highlights

| Area | What changed |
|------|----------------|
| Schema | Unchanged (**160**) |
| CI | Postgres-gated search unit tests; rustdoc private-link fix; invariant FAILED grep |
| Website | Starlight `title` frontmatter so GitHub Pages Astro build passes |
| Quickstart | No fixed `container_name`; API accepts opt-in `EDGEQUAKE_ALLOW_MOCK_PROVIDER` |

## Sequence

```text
# Already on 0.28.0 with schema 160:
docker compose pull   # or retag EDGEQUAKE_VERSION=0.28.1
docker compose up -d

# Fresh install / from older than 0.28.0:
# follow upgrade-to-0.28.0.md first (migrate → 160), then pull 0.28.1 images.
```

## Verify

```bash
curl -sf localhost:8080/health | jq '{version, schema}'
# expect version "0.28.1", schema.latest_version 160
```

## Notes

- Default LLM path remains Ollama/OpenAI. Set
  `EDGEQUAKE_LLM_PROVIDER=mock EDGEQUAKE_ALLOW_MOCK_PROVIDER=1` only for
  smoke/CI.
- [#400](https://github.com/raphaelmansuy/edgequake/issues/400),
  [#404](https://github.com/raphaelmansuy/edgequake/issues/404),
  [#405](https://github.com/raphaelmansuy/edgequake/issues/405) were closed
  against the v0.28.0 pin; this patch does not reopen them.
