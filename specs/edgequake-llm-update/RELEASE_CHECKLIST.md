# Release Checklist: edgequake-llm

> Use this checklist for every release to prevent the v0.6.15 CI publish failure
> from recurring and to ensure quality gates are met.

---

## Before writing code

- [ ] Open or reference an issue describing the change.
- [ ] Update `specs/edgequake-llm-update/EDGE_CASES.md` with any new edge case IDs.
- [ ] Add entries to `HEADER_PROPAGATION.md` or `BATCH_SIZE_LIMITS.md` if applicable.

---

## During development

- [ ] Write the test **before** the implementation (TDD).
- [ ] Ensure all new public items have doc comments.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings` locally.
- [ ] Run `cargo fmt --all` before committing.

---

## Pre-release (on feature branch)

- [ ] All `cargo test --locked` pass.
- [ ] `cargo doc --no-deps --all-features` builds without warnings.
- [ ] `cargo package --locked` succeeds (dry-run packaging).
- [ ] CHANGELOG.md has a `[x.y.z]` entry with date and detailed notes.
- [ ] `Cargo.toml` version bumped to match the new tag.

---

## Publishing protocol (MUST follow to avoid CI failure)

> **NEVER publish manually with `cargo publish` before pushing the tag.**
> The CI workflow publishes automatically when a version tag is pushed.

1. Merge PR via squash merge to `main`.
2. Pull `main` locally: `git pull origin main`.
3. **Only create the tag — do not run `cargo publish`**:
   ```bash
   git tag -a "v0.x.y" -m "Release v0.x.y"
   git push origin "v0.x.y"
   ```
4. The `publish.yml` CI workflow handles:
   - Pre-publish quality checks (fmt, clippy, build, test, doc).
   - Security audit.
   - Idempotent publish (skips if version already on crates.io).
   - GitHub release creation with CHANGELOG notes.

---

## Post-release

- [ ] Verify the GitHub Actions `publish.yml` run is green.
- [ ] Verify crates.io shows the new version: `https://crates.io/crates/edgequake-llm`.
- [ ] Update `edgequake` workspace `Cargo.toml` dependency version.
- [ ] Run `cargo update edgequake-llm` in the `edgequake` workspace.
- [ ] Build `edgequake-api` with the new version: `cargo build -p edgequake-api --lib`.
- [ ] Update and close any related issues.
- [ ] Create a `specs/edgequake-llm-update/` entry for major features.

---

## If something goes wrong

### CI publish fails with "already exists"
The version was published manually before CI ran. The v0.6.16+ CI handles this
gracefully (skips publish step). No action needed if the crate is correctly on crates.io.

### CI publish fails with "invalid token"
The `CARGO_REGISTRY_TOKEN` secret needs to be renewed in
`Settings → Secrets → Actions → CARGO_REGISTRY_TOKEN`.

### CI publish fails with "lockfile needs updating"
Run `cargo update` locally, commit `Cargo.lock`, and push a new tag.
Do NOT run with `--allow-dirty` or remove `--locked`.

### CI publish succeeds but the version is wrong
The pre-publish step `Verify tag matches Cargo.toml version` should have caught this.
If it was bypassed, yank the wrong version:
```bash
cargo yank --version x.y.z edgequake-llm
```
Then fix `Cargo.toml`, bump to a new patch, and re-tag.
