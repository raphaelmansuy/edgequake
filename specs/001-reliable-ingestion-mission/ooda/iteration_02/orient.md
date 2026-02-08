# OODA Iteration 02 - Orient

## Mission Re-Read Checkpoint
✅ Mission file re-read: `./specs/001-reliable-ingestion-mission.md`

## Analysis

### 1. gpt-4o-mini Residue Analysis

**Problem:** Iteration 01 claimed migration complete but left critical references:

| Location | Type | Risk | Action |
|----------|------|------|--------|
| `progress.rs:610-619` | Function code | Medium | Update function name to `new_gpt4o_mini_legacy()` or deprecate |
| `progress.rs:659-664` | HashMap defaults | Low | Keep for backward compatibility (valid models) |
| `progress.rs:774,798` | Tests | Low | Update to use gpt-5-nano |
| `cache.rs:297,389` | Comments/Tests | Low | Update to reflect current model |

**First Principles Analysis:**
- gpt-4o and gpt-4o-mini ARE valid OpenAI models (not deprecated, just quota exceeded)
- gpt-5-nano should be the NEW DEFAULT, not the only option
- Pricing data for old models should remain for cost tracking

**Solution Options:**
1. **Option A**: Remove ALL old model references → Risk: breaks backward compatibility
2. **Option B**: Add gpt-5-nano as default, mark old models as legacy → Balanced
3. **Option C**: Leave as-is with documentation → Minimal change

**Recommendation:** Option B - Add gpt-5-nano prominently, deprecate old model constructors

### 2. DATABASE_URL Validation Analysis

**Problem:** No runtime validation prevents accidental memory mode in production.

**Current Flow:**
```
make dev → backend-dev → db-wait → cargo run with DATABASE_URL set
```

The Makefile DOES set DATABASE_URL, but if someone runs `cargo run` directly without it, they get memory mode silently.

**First Principles Analysis:**
- Defense in depth: Multiple layers of protection are safer
- Fail-fast: Production configurations should fail early if misconfigured
- Explicit > Implicit: Make storage mode choice obvious

**Solution Options:**
1. **Option A**: Add `if [ -z "$DATABASE_URL" ]; then exit 1; fi` to Makefile dev targets
2. **Option B**: Add runtime warning in main.rs (already exists: line 254)
3. **Option C**: Add `--require-database` flag to binary

**Recommendation:** Option A + enhanced Option B (add "NOT FOR PRODUCTION" to message)

### 3. Memory Mode Documentation

**Problem:** In-memory mode is not explicitly documented as test-only.

**Current State:**
- main.rs:254 logs a warning but doesn't say "NOT FOR PRODUCTION"
- AGENTS.md mentions `--memory` but doesn't warn against production use
- Makefile has `backend-memory` target without strong warnings

**First Principles Analysis:**
- Documentation should match code behavior
- Warnings should be explicit and actionable
- Test-only features should be clearly marked

**Solution:**
1. Update main.rs warning to be more explicit
2. Add WARNING to Makefile backend-memory target
3. Update AGENTS.md to clarify memory mode limitations

### 4. Risk Assessment Matrix

| Gap | Impact | Likelihood | Priority |
|-----|--------|------------|----------|
| gpt-4o-mini in tests | Low | Medium | P3 |
| No DATABASE_URL guard | High | Low | P2 |
| Undocumented memory mode | Medium | Medium | P2 |
| Tests not verified | High | Unknown | P1 |

### 5. First Principles Summary

**Core Principle:** The ingestion pipeline must be reliable and safe by default.

**Derived Requirements:**
1. Default configuration should be production-ready
2. Test configurations should be clearly marked
3. Deprecated models should be flagged as legacy
4. Storage mode should be explicitly chosen, not implicit
5. All tests must pass before declaring success

## Recommended Action Plan

### Priority 1: Verify Tests Pass
- Run `cargo test --workspace` to completion
- Fix any failing tests before making other changes

### Priority 2: Add gpt-5-nano Support
- Add `new_gpt5_nano()` constructor (if missing)
- Add gpt-5-nano to default_model_pricing()
- Mark `new_gpt4o_mini()` as deprecated
- Update test defaults to use gpt-5-nano

### Priority 3: DATABASE_URL Safety
- Add explicit check in Makefile dev targets
- Enhance main.rs warning message
- Document memory mode as test-only in AGENTS.md

### Priority 4: Cleanup & Documentation
- Update AGENTS.md with memory mode warning
- Add WHY comments explaining storage selection logic
- Create test to verify Makefile safety

## Decision Input Required

For decide.md, I recommend:
1. **Immediate**: Run and pass all tests
2. **High Priority**: Add gpt-5-nano, deprecate gpt-4o-mini constructor
3. **Medium Priority**: Add DATABASE_URL guards
4. **Low Priority**: Documentation updates

Time estimate: 1-2 OODA iterations for P1-P2, 1 iteration for P3-P4.
