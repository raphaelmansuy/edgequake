# CoPilot Agent: Execute Plan & Verify Implementation

Purpose

- Instruct the CoPilot agent to fully execute the plan in `plan_webui_step_2/`, implement the work, and verify correctness. Emphasize frequent, small commits and a clear stabilization process when integrating two features.

Agent Goals

1. Implement features and tasks listed in the `plan_webui_step_2/` plan until all acceptance criteria are met.
2. Verify correctness with automated tests, linters, and lightweight manual checks.
3. Use a conservative, frequent-commit workflow while stabilizing integration between two features.

Branching & Commit Policy

- Create a dedicated branch per feature: `feature/<short-name>`.
- For work that integrates two features, create an integration branch: `integration/<feature-A>-<feature-B>`.
- Commit frequently: every small, logically-complete change (aim for 5–50 LOC changes per commit). Commit messages must be imperative and concise (e.g., "Add entity extraction test for X").
- Avoid monolithic commits. If a change grows large, split into logically cohesive commits before pushing.

Stabilization Between Two Features (workflow)

1. Implement each feature on its own branch and get unit tests passing locally.
2. Create an integration branch containing both feature branches merged.
3. On the integration branch, run the full test suite and the following checks:
   - `cargo test` for Rust crates
   - `cargo clippy` (fix high-severity lints)
   - `cargo fmt --check` (formatting)
   - `bun test` or `npm test` for web UI (if applicable)
   - any repo-specific integration/E2E scripts
4. Fix regressions discovered during integration on the integration branch in small commits.
5. Once tests and linters are green, open a PR from the integration branch or merge feature branches into a target branch behind PRs with CI passing.
6. If a regression is traced to a single commit, revert or fix in a follow-up commit—prefer fixes in small patches over reverts when safe.

Verification & Acceptance Criteria

- Unit tests: all pass locally and on CI.
- Integration/E2E tests: all pass for the integration branch.
- Linting: `cargo clippy` shows no errors or only acceptable warnings; address critical warnings.
- Formatting: `cargo fmt --check` passes.
- Peer review: PR description includes summary, affected areas, manual verification steps, and link to failing/tested cases.
- Smoke test: run a short manual verification script or steps (documented in PR) demonstrating the two features working together.
- Performance regressions: run quick perf check if feature affects hot paths; report any >10% degradation.

Automation & CI

- Rely on existing CI to run the full matrix. If CI is missing a critical check, add it in a small PR first.
- Before pushing large changes, run the local checks above. Push early, not late—CI gives fast feedback.

Reporting & PRs

- Every PR must include:
  - Summary of changes
  - Commands to run tests locally
  - Acceptance criteria checklist with checkboxes
  - Screenshots or logs for manual verifications when relevant
- Use draft PRs while stabilizing; convert to ready-for-review when green.

If Blocked or Unclear

- Open a short issue in the repo describing the blocker with reproduction steps and tests showing failure.
- Add a short message in the PR and tag the repo maintainer or `@raphaelmansuy` for handoff.

End-of-Work Checklist (agent must run before marking done)

- [ ] All automated tests pass locally and on CI
- [ ] Linting and formatting checks pass
- [ ] Integration branch shows no regressions between the two features
- [ ] PRs created with required details and reviewers requested
- [ ] Small, well-scoped commits exist and are descriptive
- [ ] Task/todo updates recorded in `plan_webui_step_2/` as appropriate

Notes to the executing agent

- Keep commits small and frequent. If integration shows instability, pause new feature work and focus on fixes.
- Preserve test-first mindset: add tests for bug fixes and new features.
- Update this prompt or `plan_webui_step_2/` plan with discovered gaps or follow-ups.

---

Agent: start by checking out the current plan files in `plan_webui_step_2/`, create your feature branch, and begin implementing the first prioritized item. Commit frequently and push for CI feedback. If you need clarification, open an issue and tag the maintainer.
