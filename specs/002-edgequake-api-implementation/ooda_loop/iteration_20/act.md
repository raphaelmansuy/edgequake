# Iteration 20: Java + Kotlin SDK Fix

## Observe
- Java SDK: 99 unit + 20 E2E tests already passing
- Kotlin SDK: `HttpHelper.execute()` failed on 204 empty response bodies (DELETE endpoints)
- Kotlin `Services.kt`: Conversation/folder/document delete methods tried to deserialize empty response

## Act
### Kotlin SDK Fixes
- `sdks/kotlin/src/main/kotlin/io/edgequake/sdk/internal/HttpHelper.kt`: Added empty/blank body handling in `execute()` with type-specific fallback (Unit/Map/String)
- `sdks/kotlin/src/main/kotlin/io/edgequake/sdk/resources/Services.kt`: Changed conv/folder/doc delete methods from `http.delete()` to `http.deleteRaw()`
- `sdks/kotlin/src/test/kotlin/io/edgequake/sdk/UnitTest.kt`: Updated 3 delete tests

## Results
- Java: 99 unit + 20 E2E passed, 0 skipped ✅
- Kotlin: 99 unit + 20 E2E passed, 0 skipped ✅
