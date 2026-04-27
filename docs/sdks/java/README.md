# Java SDK

**Location:** `sdks/java`

## Maven dependency

Add the SDK JAR from the project build (`mvn install`) or your internal artifact repo.

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

```bash
cd sdks/java && mvn test
```
