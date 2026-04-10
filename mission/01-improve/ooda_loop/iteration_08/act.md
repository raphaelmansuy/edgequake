# Act — Iteration 08
Date: 2026-04-10. Commit: `fd31314b`
Replaced 3 bare `.unwrap()` → `.expect()` with WHY comments in Ollama handlers and models.rs.
Verification: `cargo test -p edgequake-api --lib` → 534 passed.
