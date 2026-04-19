# Runtime Config and Authentication Hardening

EdgeQuake supports both demo-friendly local development and fail-closed authenticated deployments.

## Recommended production environment

```bash
export EDGEQUAKE_AUTH_ENABLED=true
export EDGEQUAKE_MASTER_API_KEY="replace-with-a-strong-secret"
export NEXT_PUBLIC_AUTH_ENABLED=true
export NEXT_PUBLIC_DISABLE_DEMO_LOGIN=true
export NEXT_PUBLIC_API_URL="https://your-api-host"
```

## What changed

- The WebUI now receives runtime config from the server layout rather than depending only on build-time public variables.
- Protected dashboard routes redirect to the login screen when authentication is required.
- The backend now enforces runtime auth flags and master API keys consistently.
- Bootstrap admin creation can be done securely with the configured master API key.

## Bootstrap an admin user

```bash
curl -X POST http://localhost:8080/api/v1/users \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $EDGEQUAKE_MASTER_API_KEY" \
  -d '{
    "username": "admin",
    "email": "admin@example.com",
    "password": "ChangeMe123!",
    "role": "admin"
  }'
```

## Expected behavior

### When auth is disabled

- Demo/dev flows remain available.
- Main application screens load without login.

### When auth is enabled

- Direct access to dashboard routes redirects to login.
- Demo login is hidden.
- Authenticated sessions can access the full dashboard.
- Sensitive endpoints require a valid JWT or configured API key.
