package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

/** User operations at /api/v1/users. */
public class UserService {

    private final HttpHelper http;

    public UserService(HttpHelper http) { this.http = http; }

    public UserInfo create(CreateUserRequest request) {
        return http.post("/api/v1/users", request, UserInfo.class);
    }

    public UserInfo get(String id) {
        return http.get("/api/v1/users/" + id, null, UserInfo.class);
    }

    /** WHY: Returns {users: [...]} wrapper. */
    public UserListResponse list() {
        return http.get("/api/v1/users", null, UserListResponse.class);
    }
}
