package io.edgequake.sdk.resources;

import io.edgequake.sdk.internal.HttpHelper;
import io.edgequake.sdk.models.AuthModels.*;

/** Auth operations at /api/v1/auth. */
public class AuthService {

    private final HttpHelper http;

    public AuthService(HttpHelper http) { this.http = http; }

    public TokenResponse login(LoginRequest request) {
        return http.post("/api/v1/auth/login", request, TokenResponse.class);
    }

    public UserInfo me() {
        return http.get("/api/v1/auth/me", null, UserInfo.class);
    }

    public TokenResponse refresh(RefreshRequest request) {
        return http.post("/api/v1/auth/refresh", request, TokenResponse.class);
    }
}
