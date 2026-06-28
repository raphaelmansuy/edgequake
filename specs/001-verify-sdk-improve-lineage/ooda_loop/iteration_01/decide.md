# OODA Iteration 01 - DECIDE

**Date**: 2026-02-15  
**Mission**: SDK Quality Assurance & Lineage Enhancement  
**Focus**: Prioritized Action Plan

---

## Decision: Fix Java SDK Compilation Blocker

### Chosen Solution: Downgrade to Java 17 LTS

**Rationale:**

1. User confirmed Java 17 is installed
2. Java 17 is LTS (Long-Term Support) - production stable
3. No Java 21-specific features detected in SDK codebase
4. Immediate unblock with minimal risk

---

## Action Items (This Iteration)

### Action 1: Update pom.xml for Java 17

**File**: `sdks/java/pom.xml`  
**Changes**:

```xml
<!-- BEFORE -->
<maven.compiler.source>21</maven.compiler.source>
<maven.compiler.target>21</maven.compiler.target>
...
<release>21</release>

<!-- AFTER -->
<maven.compiler.source>17</maven.compiler.source>
<maven.compiler.target>17</maven.compiler.target>
...
<release>17</release>
```

### Action 2: Build and Test Java SDK

**Commands**:

```bash
cd sdks/java
mvn clean compile
mvn test
```

**Expected**: ✅ Build passes, tests run (may have skipped E2E tests)

### Action 3: Document Java 17 Compatibility

**File**: `sdks/java/README.md`  
**Add**: Java 17+ requirement in prerequisites

---

## Deferred Actions (Next Iterations)

### Iteration 02: Java SDK Lineage Support

- Create `LineageResource.java`
- Add DTOs for lineage responses
- Write unit tests

### Iteration 03: Java SDK Metadata Support

- Add metadata parameter to document uploads
- Create metadata DTOs
- Write unit tests

### Iteration 04: Java SDK Test Coverage

- Add resource-specific tests
- Add streaming tests
- Add error handling tests

---

## Success Criteria (This Iteration)

| Criterion         | Target                             |
| ----------------- | ---------------------------------- |
| Java SDK compiles | ✅ `mvn compile` success           |
| Unit tests pass   | ✅ `mvn test` no failures          |
| No regressions    | ✅ Existing behavior preserved     |
| Documented        | ✅ README updated with Java 17 req |

---

## Risk Mitigations

| Risk                     | Mitigation                     |
| ------------------------ | ------------------------------ |
| Java 21 features in code | Audit code - none found        |
| Breaking existing users  | Semver - minor version bump    |
| CI/CD failures           | Update GitHub Actions workflow |

---

## Commit Plan

**Commit Message**: `OODA-01: Downgrade Java SDK to Java 17 LTS`

**Files Changed**:

1. `sdks/java/pom.xml` - Java version properties
2. `sdks/java/README.md` - Prerequisites section

---

## Next Steps (ACT Phase)

1. Execute pom.xml changes
2. Run `mvn clean compile`
3. Run `mvn test`
4. Update README
5. Document results in act.md
