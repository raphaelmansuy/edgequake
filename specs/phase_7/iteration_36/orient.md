# OODA-36: Orient — Go SDK Polish Strategy

## Analysis

The Go SDK has solid architecture (zero external dependencies, httptest-based testing, functional options pattern) but lacks documentation, comprehensive error path coverage, and CI.

## Priority Order

1. README.md — document all 22 services with examples and configuration reference
2. Remove dead code — unused put/patch/del inflating uncovered statement count
3. Error path tests — each service method's error return branch at 75%, add ~70 error tests
4. CI workflow — Go version matrix (1.21, 1.22, 1.23), vet, test, build
5. Verify 90%+ coverage target reached

## Approach

Follow TypeScript SDK polish pattern: comprehensive README → exhaustive tests → CI workflow → commit.
