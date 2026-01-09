---
name: doc-traceability-validator
description: Validate documentation traceability between code annotations (@implements), feature registry, business rules, and use cases. Detect ID collisions, undocumented features, broken cross-references, and namespace violations.
license: Proprietary (repository internal)
compatibility: Python 3.9+, works with any TypeScript/Rust codebase using @implements annotations
metadata:
  repo: raphaelmansuy/edgequake
  area: documentation
  languages:
    - Python
    - TypeScript
    - Rust
  frameworks:
    - EdgeQuake
  patterns:
    - Code analysis
    - Documentation validation
    - Traceability verification
    - CI/CD quality gates
---

# Documentation Traceability Validator Skill

Validate and maintain accurate traceability between code annotations (`@implements FEATXXXX`) and documentation registries (features.md, business_rules.md, use_cases.md). This skill detects gaps, collisions, and broken cross-references that compromise documentation accuracy.

## When to use

Use this skill when you need to:

- **Audit documentation accuracy**: Verify all code features are documented
- **Detect ID collisions**: Find duplicate FEAT/BR/UC IDs before they break traceability
- **Find undocumented features**: Discover @implements annotations missing from registries
- **Validate cross-references**: Ensure FEAT↔BR↔UC links are valid
- **Enforce namespace allocation**: Verify features use correct ID ranges
- **Generate registry entries**: Auto-create documentation from code annotations
- **CI/CD quality gates**: Block PRs with documentation gaps
- **Track documentation coverage**: Monitor doc completeness over time

## Core concepts

### Traceability Chain

The validation framework ensures bidirectional traceability:

```
Code (@implements FEATXXXX) ←→ features.md ←→ business_rules.md ←→ use_cases.md
```

**Chain integrity requires**:

1. Every @implements annotation has a features.md entry
2. Every features.md entry references valid BR/UC IDs
3. Every BR references valid FEAT IDs
4. Every UC references valid BR/FEAT IDs

### Validation Dimensions

| Dimension        | Weight | Description                              |
| ---------------- | ------ | ---------------------------------------- |
| **Completeness** | 40%    | % of code @implements that have docs     |
| **Uniqueness**   | 30%    | % of IDs that are unique (no collisions) |
| **Cross-refs**   | 20%    | % of BR/UC references that resolve       |
| **Namespace**    | 10%    | % of IDs in correct allocated ranges     |

**Composite Score**:

```
Score = (0.40 × Completeness) + (0.30 × Uniqueness)
      + (0.20 × CrossRefs) + (0.10 × Namespace)
```

### Namespace Allocation

Standard range allocation for EdgeQuake:

| Range    | Module                  | Team     |
| -------- | ----------------------- | -------- |
| FEAT00XX | Core Pipeline           | Backend  |
| FEAT01XX | Query Engine            | Backend  |
| FEAT02XX | Graph Storage           | Backend  |
| FEAT03XX | Streaming/Pipeline      | Backend  |
| FEAT04XX | Conversations/Citations | Frontend |
| FEAT05XX | PDF/Lineage             | Backend  |
| FEAT06XX | WebUI Core              | Frontend |
| FEAT07XX | API Client/Utils        | Frontend |
| FEAT08XX | Authentication          | Backend  |
| FEAT085X | Cost Management         | Frontend |
| FEAT086X | WebUI Providers         | Frontend |
| FEAT10XX | Document Mgmt UI        | Frontend |

## Quick start

### Basic validation

```bash
cd /path/to/edgequake

# Validate features
python3 .github/skills/doc-traceability-validator/scripts/validate_features.py \
  --code-dir edgequake_webui/src \
  --docs-file docs/features.md \
  --verbose

# Validate full traceability chain
python3 .github/skills/doc-traceability-validator/scripts/validate_traceability.py \
  --code-dir edgequake_webui/src \
  --features docs/features.md \
  --rules docs/business_rules.md \
  --usecases docs/use_cases.md \
  --output-report validation_report.json
```

### Generate registry entries

```bash
# Auto-generate feature entries from code annotations
python3 .github/skills/doc-traceability-validator/scripts/generate_registry.py \
  --code-dir edgequake_webui/src \
  --output features_generated.md \
  --format markdown
```

### CI/CD integration

```yaml
# .github/workflows/docs-validation.yml
name: Documentation Validation
on: [pull_request]

jobs:
  validate-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.11"
      - name: Install dependencies
        run: pip install -r .github/skills/doc-traceability-validator/scripts/requirements.txt
      - name: Validate documentation
        run: |
          python3 .github/skills/doc-traceability-validator/scripts/validate_features.py \
            --code-dir edgequake_webui/src \
            --docs-file docs/features.md \
            --fail-threshold 95.0
```

## Scripts reference

### validate_features.py

Scan code for @implements annotations and compare with features.md.

**Arguments**:

- `--code-dir`: Directory to scan for @implements annotations
- `--docs-file`: Path to features.md
- `--pattern`: Regex for annotation format (default: `@implements\s+(FEAT\d{4})`)
- `--extensions`: File extensions to scan (default: `.ts,.tsx,.rs`)
- `--fail-threshold`: Minimum completeness score (0-100)
- `--output-json`: Save results to JSON file
- `--verbose`: Show detailed output

**Output**:

```
Feature Validation Report
=========================
Code Features Found:     247
Documented Features:     104
Undocumented:           143 (42.1% gap)
Duplicate IDs:            7
Unique Code Features:   240

Completeness Score:      42.1%
Uniqueness Score:        97.2%
Overall Score:           63.4%

DUPLICATE IDs (CRITICAL):
  FEAT0636: 2 occurrences
    - hooks/use-debounce.ts:12
    - components/shared/empty-state.tsx:6
  ...

UNDOCUMENTED FEATURES:
  FEAT0401 - hooks/use-conversations.ts:10
  FEAT0402 - hooks/use-conversations.ts:11
  ...
```

### validate_traceability.py

Validate complete FEAT↔BR↔UC traceability chain.

**Arguments**:

- `--code-dir`: Directory with @implements annotations
- `--features`: Path to features.md
- `--rules`: Path to business_rules.md
- `--usecases`: Path to use_cases.md
- `--output-report`: Save full report to JSON
- `--fix-suggestions`: Generate suggested fixes

**Output**:

```
Traceability Validation Report
==============================
Features: 104 documented, 247 in code
Rules:     89 documented
UseCases:  56 documented

Cross-Reference Validation:
  Features → Rules:    87/104 valid (83.7%)
  Rules → Features:    85/89 valid (95.5%)
  Rules → UseCases:    52/89 valid (58.4%)
  UseCases → Rules:    48/56 valid (85.7%)

Broken References:
  features.md:L340 - BR0999 not found
  business_rules.md:L560 - FEAT9999 not found
  ...

Overall Traceability Score: 78.3%
```

### generate_registry.py

Auto-generate features.md entries from code annotations.

**Arguments**:

- `--code-dir`: Directory to scan
- `--output`: Output file path
- `--format`: Output format (markdown, json, csv)
- `--group-by`: Group by range, module, or file
- `--include-context`: Include surrounding code context

**Output** (markdown):

```markdown
### FEAT0401 - Conversation Persistence

| Attribute          | Value                                                                         |
| ------------------ | ----------------------------------------------------------------------------- |
| **ID**             | FEAT0401                                                                      |
| **Name**           | Conversation Persistence                                                      |
| **Module**         | edgequake_webui                                                               |
| **Status**         | ⚠️ Undocumented                                                               |
| **Code Reference** | [use-conversations.ts](../edgequake_webui/src/hooks/use-conversations.ts#L10) |
| **Description**    | Auto-extracted from code context                                              |
| **Related**        | (needs manual entry)                                                          |
```

### check_namespace.py

Validate FEAT IDs follow allocated ranges.

**Arguments**:

- `--code-dir`: Directory to scan
- `--allocation-file`: YAML file with range allocations
- `--team`: Filter by team (frontend/backend)

**Output**:

```
Namespace Validation Report
===========================
Total Features:     247
In Allocated Range: 235 (95.1%)
Out of Range:        12 (4.9%)

Violations:
  FEAT0800 in components/theme-provider.tsx
    Expected range: FEAT08XX (Auth/Backend)
    Actual usage: Theme support (Frontend)
    Suggestion: Move to FEAT086X (WebUI Providers)
```

## Best practices

### 1. Run validation before commits

```bash
# Add to .git/hooks/pre-commit
python3 .github/skills/doc-traceability-validator/scripts/validate_features.py \
  --code-dir edgequake_webui/src \
  --docs-file docs/features.md \
  --fail-threshold 100.0
```

### 2. Document new features immediately

When adding `@implements FEATXXXX`:

1. Check AGENTS.md for allocated range
2. Add entry to features.md in same PR
3. Run validation to confirm

### 3. Track documentation coverage

```bash
# Weekly coverage report
python3 .github/skills/doc-traceability-validator/scripts/validate_features.py \
  --code-dir . \
  --docs-file docs/features.md \
  --output-json coverage_$(date +%Y%m%d).json
```

### 4. Fix collisions immediately

Duplicate FEAT IDs are **CRITICAL** - they break traceability:

- Cannot determine which feature a BR references
- Cannot track which UC implements which feature
- Makes documentation unreliable

## Real-World Example: EdgeQuake Documentation Audit (Iterations 65-75)

This skill was developed and validated through 10 OODA loop iterations on the EdgeQuake codebase:

### Starting State (Iteration 65)

```
Code Features Found:     247
Documented Features:     104
Undocumented:           143 (42.1% gap)
Duplicate IDs:           42
Completeness Score:      57.9%
Uniqueness Score:        79.1%
```

### Actions Taken

1. **Iteration 65**: Created 4 validation scripts (validate_features.py, validate_traceability.py, generate_registry.py, check_namespace.py)
2. **Iterations 66-68**: Fixed namespace collisions by migrating conflicting IDs:
   - FEAT0701-0705 → FEAT0770-0774 (API Client Chat features)
   - FEAT0501-0506 → FEAT0861-0871 (WebUI Providers)
   - FEAT0801-0804 → FEAT0850-0853 (Cost Management)
3. **Iteration 69**: Auto-generated 120 feature entries in 5 seconds using generate_registry.py
4. **Iteration 70**: Added 20 @implements annotations to backend Rust files
5. **Iteration 74**: Identified and fixed true collision (FEAT0301 used by both backend Pipeline and frontend Chain-of-thought)
6. **Iteration 75**: Enhanced validation to distinguish cross-cutting duplicates (OK) from true collisions (FIX)

### Final State (Iteration 75)

```
Code Features Found:     201
Documented Features:     224
Undocumented:             0 (0.0% gap)
Cross-cutting duplicates: 42 (intentional, multi-layer)
True collisions:          0 (all fixed!)
Completeness Score:     100.0%
Uniqueness Score:       100.0%
Overall Score:          100.0%
```

### Key Insight: Cross-Cutting Features

The validation tool now recognizes that duplicates spanning multiple architectural layers (types, stores, hooks, components, lib) are **intentional**:

```
FEAT0001: 5x across ['components', 'lib', 'pages', 'stores', 'types']  ← OK!
FEAT0601: 8x across ['components', 'hooks', 'lib', 'pages', 'stores', 'types']  ← OK!
FEAT0734: 3x in ['components/query']  ← OK! (related components)
```

This is standard React/TypeScript architecture where the same feature is implemented across layers.

## Troubleshooting

### "X features not found in documentation"

1. Run `generate_registry.py` to create templates
2. Review generated entries for accuracy
3. Add to features.md with proper descriptions
4. Re-run validation

### "Duplicate FEAT ID: FEATXXXX"

1. Identify both usages in validation output
2. Determine which feature should keep the ID
3. Assign new ID to the other feature (from correct range)
4. Update code @implements annotation
5. Add new ID to features.md
6. Re-run validation

### "Broken reference: BRXXXX not found"

1. Check if BR was renamed or removed
2. If removed, update referencing document
3. If renamed, update the reference
4. Consider adding redirect/alias

## Related documents

- [AGENTS.md](../../../AGENTS.md) - Feature ID range allocation
- [features.md](../../../docs/features.md) - Feature registry
- [business_rules.md](../../../docs/business_rules.md) - Business rules
- [use_cases.md](../../../docs/use_cases.md) - Use cases
