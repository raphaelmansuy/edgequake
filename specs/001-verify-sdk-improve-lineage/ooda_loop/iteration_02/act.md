# OODA Iteration 02 — Act: Fix PHP, Complete Baseline

**Date**: 2026-02-13  
**Commit**: Pending

## Actions Taken

### 1. Fixed PHP SDK Autoloading (55 tests unblocked)
- **Root cause**: `Services.php` defines multiple classes but PSR-4 expects one class per file
- **Fix**: Added `classmap` to `composer.json` for `Services.php`
- **File**: `sdks/php/composer.json:13` — added `"classmap": ["src/Services.php"]`
- **Result**: 62/62 tests now pass (was 7/62)

### 2. Java/Kotlin — Requires JDK 17+ (Cannot Fix Locally)
- Code uses `Map.of()`, `var`, sealed classes — JDK 17 features
- System has JDK 8 only
- **Decision**: Note as requirement, skip in local testing

### 3. Complete Test Baseline

| SDK        | Tests | Pass | Fail | Skip | Endpoints | Coverage % |
|------------|-------|------|------|------|-----------|------------|
| Python     | 467   | 435  | 0    | 32   | 117       | ~88%       |
| TypeScript | 312   | 247  | 0    | 65   | 73        | ~55%       |  
| Rust       | 55    | 55   | 0    | 0    | 69        | ~52%       |
| C#         | 71    | 71   | 0    | 0    | 24        | ~18%       |
| Go         | 186   | 186  | 0    | 0    | 68        | ~51%       |
| Java       | —     | —    | BUILD| —    | 51        | ~38%       |
| Kotlin     | —     | —    | BUILD| —    | 32        | ~24%       |
| PHP        | 62    | 62   | 0    | 0    | 21        | ~16%       |
| Ruby       | 59    | 59   | 0    | 0    | 22        | ~17%       |
| Swift      | 70    | 70   | 0    | 0    | 26        | ~20%       |

(Coverage % = endpoints / 133 backend routes)

### Files Modified
- `sdks/php/composer.json:13` — Added classmap autoloading

## Next Focus
- Phase 2: Add missing endpoints to Python SDK (lineage export, costs)
- Start building comprehensive test coverage for Python (target: 95%)
