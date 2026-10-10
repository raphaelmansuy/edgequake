---
title: Java SDK
description: Use the EdgeQuake Java client (io.edgequake:edgequake-sdk 0.4.0). Requires Java 17. Not on Maven Central yet — install from the monorepo.
---

# Java SDK

Java client for EdgeQuake. Coordinates: **`io.edgequake:edgequake-sdk:0.4.0`**. Requires **Java 17**. **Not published** on Maven Central; build from `sdks/java`.

```bash
cd sdks/java && mvn install -DskipTests
```

```java
import io.edgequake.sdk.EdgeQuakeClient;
import io.edgequake.sdk.EdgeQuakeConfig;

var config = EdgeQuakeConfig.builder()
    .baseUrl("http://localhost:8080")
    .apiKey("eq-...")
    .build();
var client = new EdgeQuakeClient(config);

var health = client.health().check();
var docs = client.documents().list(1, 20);
```

Main class: `EdgeQuakeClient`. Services cover documents, entities, query, graph, tasks and related ops. Connections are not wrapped. See [SDK overview](../README.md).
