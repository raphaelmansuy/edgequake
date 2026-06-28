# SPEC-015: Java SDK Maven Central Publishing

This folder contains the full operational process to publish the EdgeQuake Java SDK to Maven Central with OSSRH + GPG signing in CI.

Scope:

- Java SDK publication pipeline (`sdks/java`)
- OSSRH credential setup
- GPG key generation/export/import for CI signing
- GitHub Actions secret wiring
- Release tag flow and verification
- Failure triage and recovery procedures

## Documents

- `001-why-and-scope.md` — what this process solves and what "done" means
- `002-prerequisites-and-accounts.md` — Sonatype/Central and repository prerequisites
- `003-ossrh-and-gpg-setup.md` — exact setup steps for credentials and keys
- `004-github-actions-secrets-and-workflow.md` — CI wiring and workflow behavior
- `005-release-execution-runbook.md` — release command sequence for maintainers
- `006-troubleshooting-and-recovery.md` — common errors and fixes

## Canonical implementation references

- Workflow: `.github/workflows/publish-java-sdk.yml`
- Java package config: `sdks/java/pom.xml`
- Java SDK docs: `sdks/java/README.md`
- Public SDK docs: `docs/sdks/java/README.md`

