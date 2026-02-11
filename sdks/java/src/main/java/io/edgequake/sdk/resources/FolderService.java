package io.edgequake.sdk.resources;

import com.fasterxml.jackson.core.type.TypeReference;
import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

import java.util.List;

/** Folder operations at /api/v1/folders. */
public class FolderService {

    private final HttpHelper http;

    public FolderService(HttpHelper http) { this.http = http; }

    public FolderInfo create(CreateFolderRequest request) {
        return http.post("/api/v1/folders", request, FolderInfo.class);
    }

    public List<FolderInfo> list() {
        return http.get("/api/v1/folders", null,
                new TypeReference<List<FolderInfo>>() {});
    }

    public FolderInfo get(String id) {
        return http.get("/api/v1/folders/" + id, null, FolderInfo.class);
    }

    public void delete(String id) {
        http.delete("/api/v1/folders/" + id);
    }
}
