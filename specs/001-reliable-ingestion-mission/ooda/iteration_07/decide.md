# OODA Iteration 07 - Decide

## Decision: Fix Clippy Warnings Selectively

### 1. Analysis of Getter Warning

**Warning:**
```
warning: getter function appears to return the wrong field
   --> crates/edgequake-llm/src/providers/lmstudio.rs:616:5
    |
616 |     fn model(&self) -> &str {
617 |         &self.embedding_model
```

**Finding:** This is a **false positive**. The code is CORRECT.

The `LMStudioProvider` struct has two model fields:
- `model: String` - for LLM completions
- `embedding_model: String` - for embeddings

The `EmbeddingProvider::model()` trait method correctly returns `embedding_model` because when you're using the provider as an `EmbeddingProvider`, you want the embedding model name.

**Action:** Add `#[allow(clippy::wrong_self_convention)]` with a WHY comment.

### 2. Selected Actions

| Action | Type | Risk |
|--------|------|------|
| Add allow attribute for false positive | Manual | None |
| Run auto-fix for other warnings | Auto | Low |
| Run tests | Validation | None |

### 3. Changes to Make

**lmstudio.rs (line ~616):**
```rust
impl EmbeddingProvider for LMStudioProvider {
    // ...
    
    // WHY: This is intentional - EmbeddingProvider::model() returns 
    // embedding_model (not self.model which is for LLM).
    // Clippy incorrectly suggests using self.model.
    #[allow(clippy::wrong_self_convention)]
    fn model(&self) -> &str {
        &self.embedding_model
    }
```

**Auto-fix for other warnings:**
```bash
cargo clippy --fix --allow-dirty --allow-staged
```

### 4. Test Plan

1. Apply manual fix for getter warning
2. Run `cargo clippy --fix --allow-dirty --allow-staged`
3. Run `cargo test --workspace --lib`
4. Verify `cargo clippy` shows reduced warnings

### 5. Success Criteria

- [ ] No `wrong_self_convention` warning for embedding model
- [ ] Auto-fixable warnings reduced
- [ ] All tests pass

### 6. Commit Message

```
OODA-07: Fix clippy warnings and add false positive suppression

- Add #[allow(clippy::wrong_self_convention)] to EmbeddingProvider::model()
  with WHY comment explaining it's intentional (embedding_model vs model)
- Run cargo clippy --fix for auto-fixable style improvements
- Reduce total clippy warnings from 23 to ~6

The EmbeddingProvider::model() correctly returns embedding_model because
when used as EmbeddingProvider, the embedding model name is expected.
```

## Decision Confirmed

Fix the false positive warning with an allow attribute and run auto-fix for remaining warnings.
