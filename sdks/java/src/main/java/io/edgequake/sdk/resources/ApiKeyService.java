package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

/** API key operations at /api/v1/api-keys. */
public class ApiKeyService {

    private final HttpHelper http;

    public ApiKeyService(HttpHelper http) { this.http = http; }

    public ApiKeyResponse create(String name) {
        return http.post("/api/v1/api-keys", new CreateApiKeyRequest(name), ApiKeyResponse.class);
    }

    /** WHY: Returns {keys: [...]} wrapper. */
    public ApiKeyListResponse list() {
        return http.get("/api/v1/api-keys", null, ApiKeyListResponse.class);
    }

    public void revoke(String id) {
        http.delete("/api/v1/api-keys/" + id);
    }
}
