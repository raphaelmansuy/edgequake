# OODA Iteration 01 - ORIENT

**Date**: 2026-02-15  
**Mission**: SDK Quality Assurance & Lineage Enhancement  
**Focus**: Gap Analysis & Solution Options

---

## 1. Problem Analysis: Java SDK Blocker

### Root Cause

```
Java SDK pom.xml requires JDK 21
System has JDK 17.0.18 installed
Maven compile fails with: "release version 21 not supported"
```

### Impact Assessment

| Factor                         | Impact                    |
| ------------------------------ | ------------------------- |
| Java SDK testing               | ❌ Blocked                |
| Kotlin SDK (may depend on JVM) | ⚠️ Potentially blocked    |
| CI/CD pipelines                | ⚠️ Needs JDK 21 runner    |
| Developer onboarding           | ⚠️ Extra tooling required |

### Solution Options

#### Option A: Downgrade Java SDK to Java 17

**Pros:**

- Matches installed JDK
- Broader compatibility (LTS version)
- Immediate unblock

**Cons:**

- May lose Java 21 features (virtual threads, pattern matching)
- Need to review code for 21-only features

**Risk: LOW** - Java 17 is sufficient for HTTP client SDK

#### Option B: Install Java 21 on system

**Pros:**

- Uses latest SDK features
- No code changes needed

**Cons:**

- May break other Java projects
- User asked to use Java 17

**Risk: MEDIUM** - User explicitly mentioned Java 17

#### Option C: Use SDKMAN for multiple JDKs

**Pros:**

- Flexible JDK management
- Can switch per-project

**Cons:**

- Additional tooling
- CI/CD complexity

**Recommendation: Option A** - Downgrade to Java 17 for compatibility

---

## 2. Gap Analysis: API Coverage vs SDK Implementation

### Critical Missing APIs by SDK

#### Java SDK Gaps (from routes.rs analysis)

```
❌ Metadata endpoints:
   - GET /documents/{id}/metadata
   - GET /documents/{id}/lineage
   - GET /documents/{id}/lineage/export

❌ Lineage endpoints:
   - GET /lineage/entities/{entity_name}
   - GET /lineage/documents/{document_id}
   - GET /chunks/{id}/lineage
   - GET /entities/{id}/provenance

❌ Cost endpoints:
   - GET /costs/summary
   - GET /costs/history
   - GET/PATCH /costs/budget

❌ Settings endpoints:
   - GET /settings/provider/status
   - GET /settings/providers
   - GET /models/*
```

#### Kotlin SDK Gaps (similar to Java)

Same missing endpoints as Java - likely copy-paste architecture

#### Swift SDK Gaps

```
❌ All lineage endpoints
❌ Cost management
❌ Provider settings
❌ WebSocket support
```

---

## 3. Test Coverage Analysis

### Python SDK (Reference Implementation)

```
Test Files: 17
Test Count: ~150+ assertions
Coverage Areas:
  ✅ Client initialization
  ✅ Configuration
  ✅ Pagination
  ✅ Streaming (SSE)
  ✅ Error handling
  ✅ Auth resource
  ✅ Documents resource
  ✅ Graph resource
  ✅ Conversations resource
  ✅ Query/Chat resource
  ✅ Lineage tracking
  ⚠️ E2E tests (require live backend)
```

### Java SDK Test Gap

```
Test Files: 2 (E2ETest.java, UnitTest.java)
Coverage Areas:
  ⚠️ Basic client tests only
  ❌ No resource-specific tests
  ❌ No error handling tests
  ❌ No streaming tests
  ❌ No lineage tests
  ❌ No metadata tests
```

---

## 4. Architecture Alignment

### SDK Pattern (from Python/TypeScript)

```
┌─────────────────────────────────────────────────┐
│                  SDK Client                      │
├─────────────────────────────────────────────────┤
│  Config │ Transport │ Pagination │ Streaming    │
├─────────────────────────────────────────────────┤
│                   Resources                      │
│ ┌─────┐ ┌─────┐ ┌─────────┐ ┌─────┐ ┌───────┐  │
│ │Auth │ │Docs │ │Convers. │ │Graph│ │Lineage│  │
│ └─────┘ └─────┘ └─────────┘ └─────┘ └───────┘  │
├─────────────────────────────────────────────────┤
│                   Types/DTOs                     │
└─────────────────────────────────────────────────┘
```

### Java SDK Current State

```
✅ EdgeQuakeClient
✅ EdgeQuakeConfig
✅ EdgeQuakeException
✅ internal/ (HTTP utilities)
✅ models/ (DTOs)
⚠️ resources/ (incomplete)
❌ Lineage resource
❌ Costs resource
❌ Settings resource
```

---

## 5. Risk Assessment

| Risk                                   | Probability | Impact | Mitigation                  |
| -------------------------------------- | ----------- | ------ | --------------------------- |
| Java 21 code uses unsupported features | Medium      | High   | Audit code before downgrade |
| Tests fail after downgrade             | Low         | Medium | Run full test suite         |
| Breaking changes in APIs               | Low         | High   | Keep backward compatibility |
| CI/CD needs updates                    | Medium      | Medium | Update GitHub Actions       |

---

## 6. Priority Matrix (Impact vs Effort)

```
High Impact │ ★ Java SDK Fix    │ Lineage APIs      │
            │ (Quick Win)       │ (Strategic)       │
────────────┼───────────────────┼───────────────────┤
Low Impact  │ Docs cleanup      │ Swift/PHP SDKs    │
            │ (Quick Win)       │ (Backlog)         │
────────────┴───────────────────┴───────────────────┘
            Low Effort          High Effort
```

---

## 7. Conclusions

### Key Insights

1. **Java SDK blocker is solvable** - Downgrade to Java 17 is low-risk
2. **Lineage APIs are the biggest gap** - 4+ SDKs missing lineage support
3. **Test coverage is highly uneven** - Python excellent, Java minimal
4. **Pattern exists to follow** - Python/TypeScript are reference implementations

### Recommended Order of Operations

1. Fix Java SDK compilation (immediate unblock)
2. Add lineage resources to Java SDK
3. Add tests for all Java SDK features
4. Repeat pattern for Kotlin, Go, C#
5. Leave Swift/PHP/Ruby for later phases

---

## Next Steps (DECIDE Phase)

Document specific changes to make:

1. Exact pom.xml changes for Java 17
2. List of files to create/modify for lineage support
3. Test file structure to implement
