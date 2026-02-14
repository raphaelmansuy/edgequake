error id: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/resources/EntityService.java:java/lang/String#
file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/resources/EntityService.java
empty definition using pc, found symbol in pc: java/lang/String#
empty definition using semanticdb
empty definition using fallback
non-local guesses:

offset: 693
uri: file://<WORKSPACE>/sdks/java/src/main/java/io/edgequake/sdk/resources/EntityService.java
text:
```scala
package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.GraphModels.*;

import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Entity operations at /api/v1/graph/entities.
 * WHY: Entities live under /graph/ in the real API, not at /entities/.
 */
public class EntityService {

    private final HttpHelper http;

    public EntityService(HttpHelper http) { this.http = http; }

    public EntityListResponse list(int page, int perPage, String entityType) {
        Map<String, String> params = new LinkedHashMap<>();
        if (page > 0) params.put("page", @@String.valueOf(page));
        if (perPage > 0) params.put("per_page", String.valueOf(perPage));
        if (entityType != null && !entityType.isEmpty()) params.put("entity_type", entityType);
        return http.get("/api/v1/graph/entities", params, EntityListResponse.class);
    }

    /** WHY: Returns EntityDetailResponse wrapper (entity + relationships + statistics). */
    public EntityDetailResponse get(String name) {
        return http.get("/api/v1/graph/entities/" + encode(name), null, EntityDetailResponse.class);
    }

    public CreateEntityResponse create(CreateEntityRequest request) {
        return http.post("/api/v1/graph/entities", request, CreateEntityResponse.class);
    }

    public MergeResponse merge(MergeEntitiesRequest request) {
        return http.post("/api/v1/graph/entities/merge", request, MergeResponse.class);
    }

    /**
     * WHY: Entity delete requires ?confirm=true query param or returns 422.
     * DELETE /api/v1/graph/entities/{name}?confirm=true
     */
    public EntityDeleteResponse delete(String name) {
        return http.delete("/api/v1/graph/entities/" + encode(name) + "?confirm=true",
                EntityDeleteResponse.class);
    }

    /**
     * Check if entity exists by name.
     * WHY: Uses query param entity_name, not path segment.
     */
    public EntityExistsResponse exists(String name) {
        Map<String, String> params = new LinkedHashMap<>();
        params.put("entity_name", name);
        return http.get("/api/v1/graph/entities/exists", params, EntityExistsResponse.class);
    }

    public NeighborhoodResponse neighborhood(String name, int depth) {
        Map<String, String> params = new LinkedHashMap<>();
        if (depth > 0) params.put("depth", String.valueOf(depth));
        return http.get("/api/v1/graph/entities/" + encode(name) + "/neighborhood",
                params, NeighborhoodResponse.class);
    }

    private static String encode(String value) {
        return URLEncoder.encode(value, StandardCharsets.UTF_8);
    }
}

```


#### Short summary: 

empty definition using pc, found symbol in pc: java/lang/String#