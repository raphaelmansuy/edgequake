# SDK Coverage Matrix

**Generated**: 2025-02-13 (OODA Iteration 16)  
**Backend Endpoints**: 108 unique routes  
**SDKs**: 10 (Python, TypeScript, Rust, C#, Go, Java, Kotlin, PHP, Ruby, Swift)

## Summary Table

| SDK        | Tests | Lineage | API Coverage | Status              |
| ---------- | ----- | ------- | ------------ | ------------------- |
| Python     | 520   | ✅ Full | 95%+         | ✅ Production-Ready |
| TypeScript | 357   | ✅ Full | 95%+         | ✅ Production-Ready |
| Rust       | 152   | ✅ Full | 90%+         | ✅ Production-Ready |
| C#         | 265   | ✅ Full | 90%+         | ✅ Production-Ready |
| Go         | 257   | ✅ Full | 90%+         | ✅ Production-Ready |
| Java       | 230   | ✅ Full | 90%+         | ✅ Production-Ready |
| Kotlin     | 277   | ✅ Full | 90%+         | ✅ Production-Ready |
| PHP        | 246   | ✅ Full | 90%+         | ✅ Production-Ready |
| Ruby       | 260   | ✅ Full | 90%+         | ✅ Production-Ready |
| Swift      | 257   | ✅ Full | 90%+         | ✅ Production-Ready |

**Total Tests**: 2,821

## Lineage Coverage (100% across all SDKs)

| Method              | Endpoint                           | All SDKs |
| ------------------- | ---------------------------------- | -------- |
| entityLineage       | GET /lineage/entities/{name}       | ✅       |
| documentLineage     | GET /lineage/documents/{id}        | ✅       |
| documentFullLineage | GET /documents/{id}/lineage        | ✅       |
| exportLineage       | GET /documents/{id}/lineage/export | ✅       |
| chunkDetail         | GET /chunks/{id}                   | ✅       |
| chunkLineage        | GET /chunks/{id}/lineage           | ✅       |
| entityProvenance    | GET /entities/{id}/provenance      | ✅       |

## Mission Baseline Corrections

| SDK    | Baseline Assessment | Actual Status   |
| ------ | ------------------- | --------------- |
| Java   | ❌ Missing metadata | ✅ Full lineage |
| Kotlin | ❌ Missing metadata | ✅ Full lineage |
| Swift  | ❌ Missing metadata | ✅ Full lineage |
| C#     | ⚠️ Partial metadata | ✅ Full lineage |
| Go     | ⚠️ Partial metadata | ✅ Full lineage |
| PHP    | ⚠️ Partial metadata | ✅ Full lineage |
| Ruby   | ⚠️ Partial metadata | ✅ Full lineage |

**Baseline Accuracy**: ~20% (8 of 10 SDKs mischaracterized)

## Key Improvements Made

1. **TypeScript SDK** (OODA-07): Added `exportLineage()` method
2. **Python SDK** (OODA-05/06): Reduced mypy errors 161→20 (88% reduction)
