# Contributing to EdgeQuake

EdgeQuake accepts standard open-source contributions through GitHub issues and pull requests.

## Before You Start

- Search existing [issues](https://github.com/raphaelmansuy/edgequake/issues) and [pull requests](https://github.com/raphaelmansuy/edgequake/pulls) before opening a new one.
- For bug reports, include reproduction steps, expected behavior, actual behavior, logs, and environment details.
- For larger changes, open an issue or discussion first so the direction is agreed before implementation.

## Development Setup

### Backend

```bash
git clone https://github.com/raphaelmansuy/edgequake.git
cd edgequake
make postgres-start
make dev-bg
make status
```

### Frontend

```bash
cd edgequake_webui
pnpm install
pnpm dev
```

### Optional Providers

- Ollama: `ollama serve`
- OpenAI: `export OPENAI_API_KEY="sk-..."`

## Architecture and Project Layout

- Architecture overview: `docs/architecture/overview.md`
- Crate reference: `docs/architecture/crates/README.md`
- Runtime configuration: `docs/operations/configuration.md`
- Docker and deployment: `docs/operations/docker-quickstart.md`, `docs/operations/deployment.md`
- Current feature inventory: `docs/features.md`

## Branching and Pull Requests

1. Fork the repository and create a topic branch from the default branch.
2. Make the smallest change that solves the problem.
3. Add or update tests.
4. Update documentation when behavior, configuration, or operations change.
5. Open a pull request with:
   - a concise summary
   - linked issue numbers when applicable
   - operational impact
   - test evidence

## Code Standards

- Keep functions focused and avoid duplication.
- Prefer explicit, deterministic configuration over heuristics.
- Follow Rust idioms and run:

```bash
cargo fmt
cargo clippy --workspace --all-targets
cargo test
```

- For frontend changes, also run:

```bash
cd edgequake_webui
pnpm test
```

## Testing Expectations

- Unit tests for pure logic and helpers
- Integration tests for crate boundaries
- E2E coverage for user-visible behavior when configuration, routing, or workflows change

Useful targeted commands:

```bash
cargo test -p edgequake-core
cargo test -p edgequake-api --test e2e_provider_integration
cargo test -p edgequake-api --test e2e_provider_status
```

## Documentation Expectations

Update docs when you change:

- environment variables
- Docker workflows
- API behavior
- contributor workflow

Preferred locations:

- user-facing operations: `docs/operations/`
- tutorials and migration guidance: `docs/tutorials/`
- architecture notes: `docs/architecture/`

## Finding Work

- Start with issues labeled `good first issue`, `help wanted`, or `documentation`.
- Use `docs/features.md` and the open issue tracker as the current roadmap snapshot.
- If you want to propose a larger feature, open an issue with the problem statement, expected UX/API, rollout plan, and test plan.

## Review Checklist

Before submitting, verify:

- tests pass locally for the affected area
- new env vars are documented
- Docker instructions still work
- no secrets or generated artifacts are committed

## Community Standards

By participating, you agree to follow `CODE_OF_CONDUCT.md`.
