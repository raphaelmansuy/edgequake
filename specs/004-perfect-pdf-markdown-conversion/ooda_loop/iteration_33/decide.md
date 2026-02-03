# OODA-33 DECIDE: Pivot to Quality Focus

## Decision

**Speed target is ALREADY ACHIEVED.** Pivot focus to quality improvements.

### Rationale

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Speed (per page) | <1.0s | 0.028-0.104s | ✅ ACHIEVED |
| TPS (Text Preservation) | ≥98% | 81.3% | ❌ 17% gap |
| SFS (Structure Fidelity) | ≥95% | 68.0% | ❌ 27% gap |

**Quality is now the PRIMARY bottleneck, not speed.**

## Action Items

### OODA-33 (Current)

1. ✅ Profile speed - DONE, target achieved
2. ✅ Document findings - DONE
3. ✅ Update mission status - Pending

### OODA-34+ (Next)

1. **Focus on SFS improvement** - 68% → 95% is a 27% gap
2. **Table detection quality** - Primary contributor to SFS
3. **Reading order accuracy** - Secondary contributor
4. **Font encoding edge cases** - TPS improvement

## Updated Priority Matrix

```
                        HIGH PRIORITY
                             │
    ┌────────────────────────┼────────────────────────┐
    │                        │                        │
    │   TABLE DETECTION      │    READING ORDER       │
    │   (SFS primary)        │    (SFS secondary)     │
    │                        │                        │
HIGH├────────────────────────┼────────────────────────┤MEDIUM
    │                        │                        │
    │   FONT ENCODING        │    SPEED OPTIMIZATION  │
    │   (TPS primary)        │    (ACHIEVED ✓)        │
    │                        │                        │
    └────────────────────────┼────────────────────────┘
                             │
                        LOW PRIORITY
```

## Updated Todo List

1. ✅ OODA-31: Compiler warnings fixed
2. ✅ OODA-32: Micro-tests created
3. ✅ OODA-33: Speed profiling (target achieved!)
4. → OODA-34: Table detection quality
5. → OODA-35: Reading order accuracy
6. → OODA-36: Font encoding improvements

## Mission File Update Required

Update `specs/004-perfect-pdf-markdown-conversion.md`:

```markdown
| **Speed** | <1s per page | 0.028-0.104s | ✅ ACHIEVED |
```
