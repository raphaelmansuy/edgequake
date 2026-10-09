---
title: "Ruby SDK"
description: "Ruby SDK for EdgeQuake."
---

# Ruby SDK

> **Product: v0.23.0** · SDK package: **~0.4.0** (decoupled from server version)

**Location:** `sdks/ruby`

## Status

**Tier 2 / experimental.** The gem layout is complete (`lib/edgequake/` with client, config, services). CI and unit tests exist; parity with Tier 1 (Python/TypeScript/Rust) is not guaranteed for v0.23 endpoints such as task cancel, PDF progress SSE, and `display_status` fields.

Verify critical paths against [OpenAPI](../../../edgequake_webui/openapi/openapi.snapshot.json) or Tier 1 SDKs before production use.

## Install (monorepo)

```ruby
# Gemfile
gem "edgequake", path: "../sdks/ruby"
```

```bash
cd sdks/ruby && bundle install
```

Not yet published to RubyGems as a standalone release — use path install from the monorepo.

## Example

```ruby
require "edgequake"

client = EdgeQuake::Client.new(config: EdgeQuake::Config.new(
  base_url: ENV.fetch("EDGEQUAKE_BASE_URL", "http://localhost:8080"),
  api_key:  ENV["EDGEQUAKE_API_KEY"],
  workspace_id: ENV.fetch("EDGEQUAKE_WORKSPACE_ID", "default"),
))

health = client.health.check
puts health["status"]

# Bulk conversation delete — body must use conversation_ids
# (see routes.rs / OpenAPI)
```

## v0.23 notes

- **Stateless parse (SPEC-094):** no typed wrapper yet — raw HTTP `POST /api/v1/parse` (multipart `file` + `options`; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages) + `GET /api/v1/parse/backends` + `GET /api/v1/parse/jobs/{id}`.
- Document responses expose `display_status` / `ui_phase` (SPEC-057 P4) — prefer them over raw `status`/`stage` for progress UI.
- **LLM cache (server-side):** `EDGEQUAKE_LLM_CACHE=1` default; `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE` override — no client change.

## Test

```bash
cd sdks/ruby && bundle exec rake test
# or: ruby -Ilib -Itest test/unit_test.rb
```

## See also

- In-repo reference: `sdks/ruby/README.md`
- [Brutal assessment](../BRUTAL-ASSESSMENT.md)
