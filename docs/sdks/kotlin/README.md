# Kotlin SDK

**Location:** `sdks/kotlin`

## Gradle (Kotlin DSL)

```kotlin
dependencies {
    implementation("io.edgequake:edgequake-sdk:VERSION") // or composite build / mavenLocal
}
```

## Example

```kotlin
import io.edgequake.sdk.EdgeQuakeClient
import io.edgequake.sdk.EdgeQuakeConfig

fun main() {
    val cfg = EdgeQuakeConfig(
        baseUrl = "http://localhost:8080",
        apiKey = System.getenv("EDGEQUAKE_API_KEY"),
        tenantId = System.getenv("EDGEQUAKE_TENANT_ID"),
        userId = System.getenv("EDGEQUAKE_USER_ID"),
        workspaceId = System.getenv("EDGEQUAKE_WORKSPACE_ID"),
    )
    val client = EdgeQuakeClient(cfg)

    val health = client.health.check()
    println(health.status)

    val convos = client.conversations.list()
    println(convos.size)

    val bulk = client.conversations.bulkDelete(listOf("id-1", "id-2"))
    println(bulk.affected)
}
```

## Lawful bulk delete

`bulkDelete` posts `{"conversation_ids":[...]}` and deserializes `affected` from the JSON body.

## Build & test

```bash
cd sdks/kotlin && ./gradlew test
```

See `sdks/kotlin/README.md` for the full feature list.
