package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

/** Tenant operations at /api/v1/tenants. */
public class TenantService {

    private final HttpHelper http;

    public TenantService(HttpHelper http) { this.http = http; }

    /** WHY: Returns {items: [...]} wrapper. */
    public TenantListResponse list() {
        return http.get("/api/v1/tenants", null, TenantListResponse.class);
    }

    public TenantInfo create(CreateTenantRequest request) {
        return http.post("/api/v1/tenants", request, TenantInfo.class);
    }
}
