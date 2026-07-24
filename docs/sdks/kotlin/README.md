---
title: "Kotlin SDK"
---

# Kotlin SDK

> **Product: v0.19.0** · Contract: [`openapi.snapshot.json`](../../../edgequake_webui/openapi/openapi.snapshot.json) · Spec ops: [Ingestion cancel & fairness](../../ingestion-cancel-and-fairness.md)

**Location:** `sdks/kotlin`  
**Build:** Maven only (`pom.xml`) — no Gradle wrapper in this repo.

## Maven dependency

```xml
<dependency>
    <groupId>io.edgequake</groupId>
    <artifactId>edgequake-sdk-kotlin</artifactId>
    <version>0.4.0</version>
</dependency>
```

Install to local Maven repo from source:

```bash
cd sdks/kotlin && mvn install -DskipTests
```

## Example

```kotlin
import io.edgequake.sdk.EdgeQuakeClient
import io.edgequake.sdk.EdgeQuakeConfig

fun main() {
    val client = EdgeQuakeClient(
        EdgeQuakeConfig(
            baseUrl = "http://localhost:8080",
            apiKey = System.getenv("EDGEQUAKE_API_KEY"),
            tenantId = System.getenv("EDGEQUAKE_TENANT_ID"),
            userId = System.getenv("EDGEQUAKE_USER_ID"),
            workspaceId = System.getenv("EDGEQUAKE_WORKSPACE_ID") ?: "default",
        )
    )

    val health = client.health.check()
    println(health.status)  // healthy

    val result = client.query.execute("What is EdgeQuake?")
    println(result.answer)
    result.sources.forEach { src ->
        println("${src.score} ${src.snippet?.take(80)}")
    }
}
```

Query responses expose **`answer`** and **`sources`** (not top-level chunks/entities).

## Build & test

```bash
cd sdks/kotlin && mvn test
```

E2E tests (requires running API):

```bash
cd sdks/kotlin && mvn test -Pe2e
```

## Lawful bulk delete

`client.conversations.bulkDelete(listOf("id-1", "id-2"))` posts `{"conversation_ids":[...]}` and reads `affected` from the JSON body.

## See also

- Full feature list: `sdks/kotlin/README.md`
- [SDK index](../README.md)
- [Custom Clients](../../integrations/custom-clients.md)
