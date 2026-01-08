# Iteration 41: Add WHY Comments to Undocumented Crates

## Observe
- 4 crates had 0 WHY comments: edgequake-auth, edgequake-audit, edgequake-rate-limiter, edgequake-tasks
- These crates contain key algorithmic decisions that should be documented

## Orient
- WHY comments explain non-obvious design decisions
- Help future maintainers understand rationale without reading commit history
- Target: Add comments to key algorithm choices

## Decide
Add WHY comments to:
1. jwt.rs: 30-second clock skew leeway
2. limiter.rs: Token bucket algorithm choice
3. logger.rs: Async background worker design
4. worker.rs: Worker pool architecture

## Act
Added 6 WHY comments across 4 crates explaining:
- Clock skew handling for distributed systems
- Token bucket vs alternatives (fixed/sliding window, leaky bucket)
- Unbounded channel for audit to avoid blocking API
- num_cpus worker count for CPU-bound embedding work

**Commit**: `192a535`
**Tests**: All 2,315 passing
