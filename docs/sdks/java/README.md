---
title: "Java SDK"
description: "Java SDK for EdgeQuake."
---

# Java SDK

> **Product: v0.23.0** · SDK package: **~0.4.0** (decoupled from server)

**Location:** `sdks/java`

## Maven dependency (published)

Maven Central when published via tags `sdk-java-v*`:

```xml
<dependency>
  <groupId>io.edgequake</groupId>
  <artifactId>edgequake-sdk</artifactId>
  <version>0.4.0</version>
</dependency>
```

Maintainer publication: `.github/workflows/publish-java-sdk.yml`.

## Monorepo / unreleased builds

For bleeding-edge API changes before a Maven release:

```bash
cd sdks/java && mvn install -DskipTests
```

Point your project at the local `~/.m2` artifact or use a composite build. **Server v0.23.0 may expose fields not yet in the latest published JAR** — compare OpenAPI when in doubt.

## Example

```java
var config = EdgeQuakeConfig.builder()
    .baseUrl("http://localhost:8080")
    .apiKey(System.getenv("EDGEQUAKE_API_KEY"))
    .tenantId(System.getenv("EDGEQUAKE_TENANT_ID"))
    .userId(System.getenv("EDGEQUAKE_USER_ID"))
    .workspaceId(System.getenv("EDGEQUAKE_WORKSPACE_ID"))
    .build();
var client = new EdgeQuakeClient(config);

var health = client.health().check();
System.out.println(health.status);

var affected = client.conversations().bulkDelete(List.of("c1", "c2")).affected;
System.out.println(affected);
```

## Bulk operations

- **Delete:** JSON body `{"conversation_ids":["…"]}`; response `affected` (aliases `deleted_count` / `deleted` still deserialize for older mocks).
- **Archive:** includes `archive: true`.
- **Move:** `conversation_ids` plus optional `folder_id`.

## v0.23 gaps

- Spot-check task cancel, PDF progress, and document `display_status` / `ui_phase` against OpenAPI — Java models may trail Tier 1.
- **Stateless parse (SPEC-094):** no typed `parse` client yet — use raw HTTP `POST /api/v1/parse` (multipart; sync ≤ 15 pages / 20 MiB, async ≤ 1000 pages via `async: true` + poll `GET /api/v1/parse/jobs/{id}`), or `GET /api/v1/parse/backends`.
- **LLM cache (server-side):** `EDGEQUAKE_LLM_CACHE=1` default; overrides `EDGEQUAKE_KEYWORD_CACHE` / `EDGEQUAKE_QUERY_ANSWER_CACHE`. No client field — set on the server.

```bash
cd sdks/java && mvn test
```

## See also

- In-repo reference: `sdks/java/README.md`
- [Brutal assessment](../BRUTAL-ASSESSMENT.md)
