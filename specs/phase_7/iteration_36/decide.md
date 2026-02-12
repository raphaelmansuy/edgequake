# OODA-36: Decide — Go SDK Actions

## Decisions

1. Create README.md covering all 22 services, configuration options, error handling, retry behavior
2. Remove unused private methods put(), patch(), del() from client.go
3. Add ~70 error path tests covering every service method's error return branch
4. Add retry body resend test (GetBody path in do())
5. Create CI workflow with Go 1.21/1.22/1.23 matrix
6. Target: 90%+ coverage, 190+ tests

## Risk Assessment

- Low risk: All changes are additive (tests, docs) or dead code removal
- No API surface changes
