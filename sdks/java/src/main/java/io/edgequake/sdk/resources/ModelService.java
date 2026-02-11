package io.edgequake.sdk.resources;

import com.fasterxml.jackson.core.type.TypeReference;
import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.OperationModels.*;

import java.util.List;

/**
 * Model/Provider operations at /api/v1/models and /api/v1/settings.
 *
 * WHY: GET /api/v1/models returns ProviderCatalog {providers: [...]}.
 * GET /api/v1/models/health returns bare array of ProviderHealthInfo.
 * GET /api/v1/settings/provider/status returns ProviderStatus.
 */
public class ModelService {

    private final HttpHelper http;

    public ModelService(HttpHelper http) { this.http = http; }

    /** GET /api/v1/models → ProviderCatalog */
    public ProviderCatalog list() {
        return http.get("/api/v1/models", null, ProviderCatalog.class);
    }

    /** WHY: Provider status is at /api/v1/settings/provider/status (not /models/status). */
    public ProviderStatus providerStatus() {
        return http.get("/api/v1/settings/provider/status", null, ProviderStatus.class);
    }

    /** WHY: Returns bare array (not wrapped in object). */
    public List<ProviderHealthInfo> providerHealth() {
        return http.get("/api/v1/models/health", null,
                new TypeReference<List<ProviderHealthInfo>>() {});
    }
}
