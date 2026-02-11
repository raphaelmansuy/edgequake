# Iteration 13 — Orient: Go SDK Architecture

## Analysis

### SDK Structure (Flat Package)

```
sdks/go/
├── go.mod              # Module: github.com/edgequake/edgequake-go, Go 1.21
├── doc.go              # Package documentation
├── option.go           # Functional options (WithBaseURL, WithAPIKey, etc.)
├── error.go            # Sentinel errors + APIError type with errors.Is support
├── types.go            # ~60 struct types for all API domains
├── client.go           # Client with NewClient(), HTTP methods, retry logic
├── services.go         # All 22 service types (methods for each endpoint)
└── edgequake_test.go   # 55 tests using net/http/httptest
```

### Key Architecture Decisions

1. **Flat package** — single `edgequake` package, no sub-packages. Idiomatic for Go client libraries.
2. **All services in one file** — avoids `create_file` tooling issues with duplicate package declarations. Services.go consolidates 22 service types.
3. **Functional options** — `NewClient(WithBaseURL("..."), WithAPIKey("..."))` pattern.
4. **Retry with exponential backoff** — 3 retries by default, jitter-free, only retries 429/5xx.
5. **Sentinel errors** — `ErrNotFound`, `ErrUnauthorized`, etc. with `APIError.Is()` for `errors.Is()` matching.
6. **Zero dependencies** — `net/http`, `encoding/json`, `context`, `fmt`, `io`, `time`, `errors`, `net/url`, `bytes`, `strings`.

### Risk Assessment

- **Low risk**: Go stdlib is extremely stable and well-documented
- **Medium risk**: SSE streaming not implemented (deferred to future iteration)
- **Low risk**: Test coverage is good at 55 tests covering all 22 services
