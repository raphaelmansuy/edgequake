# EdgeQuake Python SDK - Authentication Guide

Complete guide to authentication methods supported by the EdgeQuake Python SDK.

## Table of Contents

- [Authentication Methods](#authentication-methods)
- [API Key Authentication](#api-key-authentication)
- [JWT Token Authentication](#jwt-token-authentication)
- [Multi-Tenant Authentication](#multi-tenant-authentication)
- [Security Best Practices](#security-best-practices)

---

## Authentication Methods

EdgeQuake supports three primary authentication methods:

1. **API Key** — Simple key-based authentication (recommended for server-to-server)
2. **JWT Tokens** — Login with email/password, receive tokens (recommended for user-facing apps)
3. **Multi-Tenant** — Workspace/tenant scoped authentication

---

## API Key Authentication

API key authentication is the simplest method, ideal for server-to-server communication and automation.

### Basic Usage

```python
from edgequake import EdgequakeClient

# Method 1: Environment variable (recommended)
import os
client = EdgequakeClient(
    api_key=os.environ.get("EDGEQUAKE_API_KEY")
)

# Method 2: Explicit parameter (not recommended for production)
client = EdgequakeClient(
    api_key="your-api-key-here"
)
```

### Environment Variable

Set the API key in your environment:

```bash
export EDGEQUAKE_API_KEY="sk-your-api-key-here"
python your_script.py
```

Or use a `.env` file:

```bash
# .env
EDGEQUAKE_API_KEY=sk-your-api-key-here
```

Then load it:

```python
from dotenv import load_dotenv
load_dotenv()

from edgequake import EdgequakeClient
client = EdgequakeClient()  # Reads from environment
```

### How It Works

The SDK adds the API key to the `Authorization` header:

```http
GET /health HTTP/1.1
Host: localhost:8080
Authorization: Bearer sk-your-api-key-here
```

### Obtaining an API Key

API keys are typically issued by the EdgeQuake server administrator:

```bash
# Example: Generate API key via CLI
edgequake-cli api-keys create --name "Production Server"
# Output: sk-proj-abc123def456...
```

Or via the admin API:

```python
# Admin client
admin = EdgequakeClient(api_key="admin-key")

# Create new API key
key = admin.api_keys.create(name="Production", scopes=["read", "write"])
print(key["key"])  # sk-proj-...
```

---

## JWT Token Authentication

JWT (JSON Web Token) authentication is ideal for user-facing applications where users log in with credentials.

### Login Flow

```python
from edgequake import EdgequakeClient

# Step 1: Create client without authentication
client = EdgequakeClient()

# Step 2: Login with email/password
auth_response = client.auth.login(
    email="user@example.com",
    password="SecurePassword123!"
)

print(auth_response)
# {
#   "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
#   "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
#   "token_type": "Bearer",
#   "expires_in": 3600
# }

# Step 3: Create authenticated client
authenticated_client = EdgequakeClient(
    api_key=auth_response["access_token"]
)

# Now use the authenticated client
docs = authenticated_client.documents.list()
```

### Token Refresh

JWTs expire after a set period (typically 1 hour). Use the refresh token to get a new access token:

```python
# Before access token expires
new_auth = client.auth.refresh(
    refresh_token=auth_response["refresh_token"]
)

# Update client with new token
authenticated_client = EdgequakeClient(
    api_key=new_auth["access_token"]
)
```

### Automatic Token Refresh (Advanced)

```python
import time
from edgequake import EdgequakeClient
from edgequake.exceptions import UnauthorizedError

class AutoRefreshClient:
    def __init__(self, email, password):
        self.email = email
        self.password = password
        self._login()

    def _login(self):
        temp_client = EdgequakeClient()
        auth = temp_client.auth.login(self.email, self.password)
        self.access_token = auth["access_token"]
        self.refresh_token = auth["refresh_token"]
        self.expires_at = time.time() + auth["expires_in"]
        self.client = EdgequakeClient(api_key=self.access_token)

    def _ensure_fresh_token(self):
        # Refresh 5 minutes before expiry
        if time.time() > self.expires_at - 300:
            temp_client = EdgequakeClient()
            auth = temp_client.auth.refresh(self.refresh_token)
            self.access_token = auth["access_token"]
            self.refresh_token = auth.get("refresh_token", self.refresh_token)
            self.expires_at = time.time() + auth["expires_in"]
            self.client = EdgequakeClient(api_key=self.access_token)

    def __getattr__(self, name):
        self._ensure_fresh_token()
        return getattr(self.client, name)

# Use auto-refreshing client
client = AutoRefreshClient("user@example.com", "password")
docs = client.documents.list()  # Auto-refreshes if needed
```

---

## Multi-Tenant Authentication

Multi-tenant authentication adds workspace and tenant context to every request.

### Workspace Context

```python
from edgequake import EdgequakeClient

# Client scoped to a workspace
client = EdgequakeClient(
    api_key="your-api-key",
    workspace_id="workspace-123"
)

# All operations are scoped to this workspace
docs = client.documents.list()  # Only returns docs in workspace-123
```

### Tenant + Workspace Context

```python
# Full multi-tenant setup
client = EdgequakeClient(
    api_key="your-api-key",
    tenant_id="tenant-456",
    workspace_id="workspace-123"
)

# All operations scoped to tenant-456, workspace-123
doc = client.documents.upload(
    content="Tenant-specific document",
    title="Scoped Doc"
)
```

### How It Works

The SDK adds headers to every request:

```http
POST /documents HTTP/1.1
Host: localhost:8080
Authorization: Bearer your-api-key
X-Tenant-Id: tenant-456
X-Workspace-Id: workspace-123
Content-Type: application/json

{"content": "...", "title": "..."}
```

### Switching Workspaces

```python
# Production workspace
prod_client = EdgequakeClient(
    api_key="api-key",
    workspace_id="prod-workspace"
)
prod_docs = prod_client.documents.list()

# Development workspace
dev_client = EdgequakeClient(
    api_key="api-key",
    workspace_id="dev-workspace"
)
dev_docs = dev_client.documents.list()
```

---

## Security Best Practices

### 1. Never Hardcode Credentials

❌ **Bad:**

```python
client = EdgequakeClient(api_key="sk-proj-abc123...")
```

✅ **Good:**

```python
import os
client = EdgequakeClient(api_key=os.environ.get("EDGEQUAKE_API_KEY"))
```

### 2. Use Environment Variables

Store secrets in environment variables, not source code:

```bash
# .bashrc or .zshrc
export EDGEQUAKE_API_KEY="sk-..."

# Or use .env file (never commit it!)
# .gitignore should include:
.env
```

### 3. Rotate API Keys Regularly

```python
# Rotate keys every 90 days
admin = EdgequakeClient(api_key="admin-key")

# Revoke old key
admin.api_keys.revoke("old-key-id")

# Create new key
new_key = admin.api_keys.create(name="Q1-2025")
print(new_key["key"])  # Update your .env
```

### 4. Limit Key Scopes

Create API keys with minimal required permissions:

```python
# Read-only key for analytics
analytics_key = admin.api_keys.create(
    name="Analytics",
    scopes=["read"]
)

# Full access for automation
automation_key = admin.api_keys.create(
    name="Automation",
    scopes=["read", "write", "admin"]
)
```

### 5. Use HTTPS in Production

❌ **Bad (development only):**

```python
client = EdgequakeClient(base_url="http://api.example.com")
```

✅ **Good (production):**

```python
client = EdgequakeClient(base_url="https://api.example.com")
```

### 6. Handle Expired Tokens Gracefully

```python
from edgequake.exceptions import UnauthorizedError

try:
    docs = client.documents.list()
except UnauthorizedError:
    # Token expired or invalid
    # Option 1: Refresh token
    new_auth = client.auth.refresh(refresh_token)
    client = EdgequakeClient(api_key=new_auth["access_token"])

    # Option 2: Re-login
    auth = client.auth.login(email, password)
    client = EdgequakeClient(api_key=auth["access_token"])
```

### 7. Secure Token Storage

For client-side apps (not recommended for sensitive data):

```python
import keyring

# Store token in OS keyring (macOS Keychain, Windows Credential Manager, etc.)
keyring.set_password("edgequake", "access_token", auth["access_token"])

# Retrieve later
access_token = keyring.get_password("edgequake", "access_token")
client = EdgequakeClient(api_key=access_token)
```

### 8. Audit API Key Usage

```python
# Monitor API key usage
admin = EdgequakeClient(api_key="admin-key")

# List all keys
keys = admin.api_keys.list()
for key in keys.get("items", []):
    print(f"{key['name']}: Last used {key['last_used_at']}")

# Revoke unused keys
if key["last_used_at"] is None:
    admin.api_keys.revoke(key["id"])
```

---

## Troubleshooting

### Unauthorized Errors (401)

**Problem:** `HTTP 401: Unauthorized`

**Solutions:**

1. Check API key is set: `echo $EDGEQUAKE_API_KEY`
2. Verify key hasn't been revoked
3. Ensure key has required scopes
4. Check JWT token hasn't expired

### Forbidden Errors (403)

**Problem:** `HTTP 403: Forbidden`

**Solutions:**

1. Verify API key has required permissions
2. Check workspace/tenant context is correct
3. Ensure user has access to resource

### Token Expiration

**Problem:** Token expires during long-running operations

**Solution:** Use refresh token proactively:

```python
import time

def query_with_refresh(client, auth_response):
    start_time = time.time()
    # Refresh 5 minutes before expiry
    if time.time() - start_time > auth_response["expires_in"] - 300:
        new_auth = client.auth.refresh(auth_response["refresh_token"])
        client = EdgequakeClient(api_key=new_auth["access_token"])
        auth_response = new_auth
    return client.query.execute(query="...")
```

---

## See Also

- **API Reference:** [`API.md`](API.md)
- **Streaming Guide:** [`STREAMING.md`](STREAMING.md)
- **Examples:** [`examples/multi_tenant.py`](../examples/multi_tenant.py)
