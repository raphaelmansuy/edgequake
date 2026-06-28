# OODA Iteration 01 - ACT

**Date**: 2026-02-15  
**Mission**: SDK Quality Assurance & Lineage Enhancement  
**Focus**: Java SDK Java 17 Compatibility Fix

---

## Changes Implemented

### 1. pom.xml Updates

**File**: `sdks/java/pom.xml`

```diff
- <maven.compiler.source>21</maven.compiler.source>
- <maven.compiler.target>21</maven.compiler.target>
+ <!-- WHY: Java 17 LTS for broad compatibility (OODA-01) -->
+ <maven.compiler.source>17</maven.compiler.source>
+ <maven.compiler.target>17</maven.compiler.target>

- <release>21</release>
+ <!-- WHY: Java 17 LTS for broad compatibility (OODA-01) -->
+ <release>17</release>
```

**Lines Changed**: 24-25, 62

### 2. Test File Updates

**Files**:

- `sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java`
- `sdks/java/src/test/java/io/edgequake/sdk/E2ETest.java`
- `sdks/java/src/test/java/io/edgequake/sdk/FakeHttpClient.java`

**Change**: Replaced Java 21 SequencedCollection methods with Java 17 alternatives

```diff
- list.getFirst()
+ list.get(0)

- captured.getLast()
+ captured.get(captured.size() - 1)
```

**Total Replacements**: 28 occurrences across 3 files

---

## Build Verification

### Environment

```text
JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
Java Version: 17.0.18 (OpenJDK Homebrew)
Maven Version: 3.9.11
```

### Compile Result

```text
[INFO] Compiling 33 source files with javac [debug release 17] to target/classes
[INFO] BUILD SUCCESS
Total time: 0.637 s
```

### Test Result

```text
[INFO] Running io.edgequake.sdk.UnitTest
[INFO] Tests run: 230, Failures: 0, Errors: 0, Skipped: 0, Time elapsed: 1.130 s
[INFO] BUILD SUCCESS
Total time: 2.785 s
```

**Status**: ✅ All 230 unit tests pass

---

## Commit

**Message**: `OODA-01: Downgrade Java SDK to Java 17 LTS`

**Files Changed**:

1. `sdks/java/pom.xml`
2. `sdks/java/src/test/java/io/edgequake/sdk/UnitTest.java`
3. `sdks/java/src/test/java/io/edgequake/sdk/E2ETest.java`
4. `sdks/java/src/test/java/io/edgequake/sdk/FakeHttpClient.java`

---

## Impact Assessment

| Metric       | Before    | After                     |
| ------------ | --------- | ------------------------- |
| Java Version | 21        | 17                        |
| Compilation  | ❌ Failed | ✅ Success                |
| Unit Tests   | N/A       | ✅ 230 pass               |
| E2E Tests    | Skipped   | Skipped (require backend) |

---

## Next Iteration Focus

1. Create SDK coverage matrix for all 131+ API endpoints
2. Identify missing lineage/metadata APIs in Java SDK
3. Add lineage resource to Java SDK
4. Verify Java 17 compatibility in GitHub Actions workflow

---

## Lessons Learned

1. **Java 21 `getFirst()`/`getLast()`**: These SequencedCollection methods are new in Java 21. For Java 17 compatibility, use `get(0)` and `get(size()-1)`.

2. **Maven JAVA_HOME**: Maven uses JAVA_HOME, not necessarily the `java` on PATH. Always verify with `mvn -v`.

3. **LTS Preference**: Java 17 LTS is widely deployed; targeting it increases SDK adoption.

---

## Verification Commands

```bash
# Set Java 17
export JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home

# Compile
cd sdks/java && mvn clean compile

# Test
mvn test

# Expected: BUILD SUCCESS, 230 tests pass
```
