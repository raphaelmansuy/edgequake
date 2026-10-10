---
title: Kotlin SDK
description: Use the EdgeQuake Kotlin client (io.edgequake:edgequake-sdk-kotlin 0.4.0). JVM 17. Not on Maven Central yet.
---

# Kotlin SDK

Kotlin client for EdgeQuake. Coordinates: **`io.edgequake:edgequake-sdk-kotlin:0.4.0`**. Targets **JVM 17**. **Not published** on Maven Central; install from `sdks/kotlin`.

```bash
cd sdks/kotlin && mvn install -DskipTests
```

```kotlin
import io.edgequake.sdk.EdgeQuakeClient
import io.edgequake.sdk.EdgeQuakeConfig

val client = EdgeQuakeClient(
    EdgeQuakeConfig(baseUrl = "http://localhost:8080", apiKey = "eq-...")
)
val docs = client.documents.list(page = 1, pageSize = 20)
```

Class: `EdgeQuakeClient`. Same general surface as the Java SDK. Connections are not wrapped. See [SDK overview](../README.md).
