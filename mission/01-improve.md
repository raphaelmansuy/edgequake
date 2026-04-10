# Mission: EdgeQuake Improvement Mission

## Objective

Improve EdgeQuake with real, verifiable work only.

The target state is:

- lower code smell
- stronger SRP, DRY, and SOLID adherence
- better test reliability and coverage
- fewer flaky behaviors
- higher-signal code comments that explain WHY, tradeoffs, and architecture
- evidence that changes were actually implemented and tested

This mission is executed inside `mission/01-improve`.

Mission file:

- `mission/01-improve.md`

Read this file at the start of every iteration.

## Non-Negotiable Rules

These rules exist because previous work violated the mission.

1. Never fabricate progress.
2. Never create fake OODA iterations, fake summaries, fake commits, fake tests, fake line references, or fake completion claims.
3. Never pre-generate iteration folders or documents for work that has not happened yet.
4. Never backfill invented history to make the mission look complete.
5. Never claim a file was changed, a command was run, or a test passed unless that actually happened in the current repository state.
6. Never write `summary.md` from imagination. It must be derived from real completed iterations only.
7. Never continue code edits if the current iteration does not already have its required OODA files created first.
8. If you discover prior mission violations, stop, document the truth, remove fake artifacts if needed, and only then continue.

## Definition Of A Real Iteration

One OODA iteration is real only if all of the following are true:

1. The mission file was re-read before the iteration.
2. A new iteration directory was created before any new code edits for that iteration:
   - `observe.md`
   - `orient.md`
   - `decide.md`
   - `act.md`
3. The content of those files is specific to the actual repository state at that moment.
4. The iteration has a distinct objective.
5. The work changes real files or produces real verification evidence.
6. `act.md` records what was actually changed, tested, and observed.

If any of the above is missing, the iteration is not valid.

## Minimum Execution Requirement

Execute at least 50 real OODA iterations.

That means:

- 50 distinct iteration directories
- 4 real files per iteration
- real code or documentation work per iteration
- real verification per iteration
- no templated filler

Small iterations are acceptable if they are real, specific, and verified.

## Required Directory Structure

```text
mission/01-improve/
├── ooda_loop/
│   ├── iteration_01/
│   │   ├── observe.md
│   │   ├── orient.md
│   │   ├── decide.md
│   │   └── act.md
│   ├── iteration_02/
│   │   ├── observe.md
│   │   ├── orient.md
│   │   ├── decide.md
│   │   └── act.md
│   └── ...
└── summary.md
```

## Mandatory Iteration Order

The order below is strict.

For every iteration:

1. Re-read `mission/01-improve.md`.
2. Inspect the current repo state.
3. Create the new iteration directory.
4. Write `observe.md`.
5. Write `orient.md`.
6. Write `decide.md`.
7. Only after steps 1-6 are complete, edit code or documentation for that iteration.
8. Run verification relevant to the changes.
9. Write `act.md` with real evidence.
10. Commit if appropriate.
11. Move to the next iteration and repeat from step 1.

No code edits for an iteration may happen before its 4 OODA files exist.

## Per-File Requirements

### `observe.md`

Must contain verified facts only:

- files inspected
- current behavior
- failing tests or lints
- dependency relationships
- actual file paths
- actual line references when available
- command evidence that informed the iteration

Do not put solutions here unless they are observations from the codebase itself.

### `orient.md`

Must analyze:

- root cause hypotheses grounded in observed code
- tradeoffs
- risks
- first-principles reasoning
- alternative approaches considered

Include why some options were rejected.

### `decide.md`

Must state:

- the exact scope of this iteration
- what will be changed now
- what will not be changed now
- why this slice has the best risk-adjusted impact
- what verification will be run

The decision must be narrow enough to verify.

### `act.md`

Must record only completed work:

- files changed
- actual edits made
- actual commands run
- actual verification results
- actual line references
- actual commit SHA if a commit was created

If changes are still uncommitted, say `working tree (uncommitted)` instead of inventing a SHA.

## Evidence Standard

Every iteration must leave evidence.

Acceptable evidence includes:

- exact file paths
- exact line numbers
- exact command invocations
- real test names
- real lint output summaries
- real commit SHAs
- real diffs in the working tree

Unacceptable evidence includes:

- generic summaries without file references
- guessed line numbers
- placeholder SHAs
- “tests passed” without naming the tests or commands
- “implemented” without diff evidence

## Recovery Rule After A Process Violation

If the repository already contains code edits that were made before the iteration OODA files:

1. Stop making further code edits.
2. Create the next valid iteration directory immediately.
3. Use `observe.md`, `orient.md`, and `decide.md` to describe the actual current state honestly.
4. Use `act.md` to document the already-present working tree changes truthfully.
5. Only after that recovery iteration is documented may new code edits continue.

Do not hide the violation. Document it accurately and recover cleanly.

## Real Testing Requirement

You must test after changes.

For each iteration, record:

- the command run
- what it validates
- whether it passed or failed
- what the next action is if it failed

At mission completion, provide evidence that the relevant tests and checks pass.

Examples of acceptable verification:

- `cargo test -p edgequake-api --test e2e_rebuild_lineage`
- `cargo clippy -p edgequake-api --all-targets -- -D warnings`
- `cargo fmt --check`
- targeted frontend tests when frontend files are changed

## Code Quality Requirements

All changes must aim to improve one or more of the following:

- SRP
- DRY
- explicitness
- maintainability
- test clarity
- determinism
- failure handling
- comment signal quality

Keep functions focused. Split large responsibilities when justified. Avoid speculative refactors that increase risk without evidence.

## Documentation Requirements

When documentation or comments are updated:

- explain WHY, not only WHAT
- explain tradeoffs when relevant
- keep wording precise
- use ASCII diagrams only when they add real clarity
- avoid verbose filler

## Commit Rules

Commits must also be real.

- Do not mention a commit in `act.md` until it exists.
- Use iteration-aligned subjects when appropriate, for example:
  - `OODA-01: document recovery iteration for mission 01`
  - `OODA-02: fix clippy warnings in rebuild lineage tests`
- If the work should remain uncommitted, say so explicitly.

## Summary Rules

`mission/01-improve/ooda_loop/summary.md` may be created or updated only from real completed iterations.

It must include:

- iteration numbers that actually exist
- themes that actually recurred
- risks still open
- evidence-backed lessons

It must not:

- describe nonexistent iterations
- claim mission completion early
- hide failed attempts

## Completion Criteria

The mission is complete only when all of the following are true:

1. At least 50 real iterations exist.
2. Every iteration has all 4 required files.
3. Each iteration corresponds to real work and real verification.
4. The codebase is materially improved.
5. Relevant tests and checks have been run with recorded evidence.
6. `summary.md` reflects only real iterations.

If any item above is false, the mission is not complete.

## Operating Principle

Map the territory first.

Do not assume.
Do not simulate.
Do not narrate imaginary progress.
Verify against the repository, then act.
