# SPEC-009 — Implementation proof (executable)

**Authority:** `edgequake/crates/edgequake-api/src/routes.rs`  
**Goal:** Demonstrate that SDK work is real (tests + Docker-backed E2E where applicable).

## 1. Primary SDKs (quality gate: **PASS**)

| SDK | Unit / integration | Live E2E |
|-----|-------------------|----------|
| Rust | `cd sdks/rust && cargo test -p edgequake-sdk` | `EDGEQUAKE_BASE_URL=<url> cargo test -p edgequake-sdk --test e2e_tests --features e2e` |
| Python | `cd sdks/python && pytest` | `EDGEQUAKE_E2E_URL=<url> pytest tests/test_e2e.py -v` |
| TypeScript | `cd sdks/typescript && bun test` | `EDGEQUAKE_E2E_URL=<url> bun test tests/e2e` |

## 2. Docker Compose → SDK E2E (Makefile)

One command after images are available:

```bash
make sdk-e2e-with-stack
```

This runs `make stack` (root `docker-compose.quickstart.yml`), waits for `GET /health`, then runs Rust (`--features e2e`), Python (`tests/test_e2e.py`), and TypeScript (`tests/e2e`) against `SDK_E2E_URL` (default `http://localhost:8080`).

**Override API URL:**

```bash
make sdk-e2e SDK_E2E_URL=http://127.0.0.1:9090
```

**Prerequisites:** Docker daemon healthy; ports 8080 (API) and 3000 (UI) free or consistent with your compose file.

## 3. Secondary SDKs (quality gate: **PARTIAL** — see `SDK-QUALITY-ASSESSMENT.md`)

Run language-specific tests when toolchains are installed:

| SDK | Command (typical) |
|-----|-------------------|
| Go | `cd sdks/go && go test ./...` |
| C# | `cd sdks/csharp && dotnet test --filter "E2E!=true"` (unit); with API: `EDGEQUAKE_E2E=1 dotnet test --filter "E2E=true"` |
| Java | `cd sdks/java && mvn test` |
| Kotlin | `cd sdks/kotlin && mvn test` |
| Ruby | `cd sdks/ruby && ruby -Ilib:test test/unit_test.rb` | Optional: point `EDGEQUAKE_BASE_URL` at a live API for manual calls |
| Swift | `cd sdks/swift && swift test` | Live API: set `EDGEQUAKE_E2E=1` (and optional `EDGEQUAKE_BASE_URL`); default `swift test` skips E2E |
| PHP | See `sdks/php` README / test runner |

## 4. Evidence checklist (maintainers)

- [ ] `make sdk-e2e` exits 0 against a compose-started API  
- [ ] `cargo test -p edgequake-sdk`, `pytest`, `bun test` exit 0 without E2E env (skipped E2E is OK)  
- [ ] `specs/009-skd-update/SDK-QUALITY-ASSESSMENT.md` updated when `routes.rs` changes  

## 5. Git tracking for this spec tree

`.gitignore` uses:

```gitignore
/specs/*
!/specs/009-skd-update/
!/specs/009-skd-update/**
```

So curated SPEC-009 documents remain versioned while other local `specs/` drafts stay ignored.
