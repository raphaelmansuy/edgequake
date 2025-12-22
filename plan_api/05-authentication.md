# Authentication & Authorization Specification

**Version:** 1.0  
**Target Release:** EdgeQuake v2.0.0  
**Priority:** HIGH (Production)  
**Status:** Planning

---

## Overview

Implement JWT-based authentication and API key authentication to secure EdgeQuake API endpoints.

### Goals

1. **JWT Authentication:** Secure token-based auth for user sessions
2. **API Key Authentication:** Simple API key support for service-to-service
3. **Combined Auth:** Support both methods on same endpoints
4. **RBAC:** Role-based access control (admin, user, readonly)
5. **Security:** Industry-standard practices (bcrypt, secure tokens)

---

## Authentication Methods

### 1. JWT (JSON Web Token)

**Login Endpoint:**
```http
POST /api/v1/auth/token
Content-Type: application/json

{
  "username": "user@example.com",
  "password": "secure_password"
}
```

**Response (200 OK):**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "user": {
    "user_id": "user-123",
    "username": "user@example.com",
    "role": "user"
  }
}
```

**Using JWT:**
```http
GET /api/v1/documents HTTP/1.1
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 2. API Key

**API Key Header:**
```http
GET /api/v1/documents HTTP/1.1
X-API-Key: sk_live_abc123def456ghi789jkl012
```

**Or Query Parameter (discouraged for security):**
```http
GET /api/v1/documents?api_key=sk_live_abc123def456ghi789jkl012
```

---

## Data Models

### User Schema

```sql
CREATE TABLE users (
    user_id VARCHAR(100) PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    metadata JSONB,
    CONSTRAINT valid_role CHECK (role IN ('admin', 'user', 'readonly'))
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
```

### API Key Schema

```sql
CREATE TABLE api_keys (
    key_id VARCHAR(100) PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    key_hash TEXT NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(255),
    scopes TEXT[],
    rate_limit_tier VARCHAR(20),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    metadata JSONB,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_api_keys_user ON api_keys(user_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
```

### Refresh Token Schema

```sql
CREATE TABLE refresh_tokens (
    token_id VARCHAR(100) PRIMARY KEY,
    user_id VARCHAR(100) NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
```

---

## Rust Implementation

```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub username: String,
    pub role: String,
    pub exp: usize,   // Expiry timestamp
    pub iat: usize,   // Issued at
}

pub struct AuthService {
    jwt_secret: String,
    jwt_expiry_hours: i64,
}

impl AuthService {
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<TokenResponse, Error> {
        // Get user from database
        let user = self.user_storage
            .get_by_username(username)
            .await?
            .ok_or(Error::InvalidCredentials)?;
        
        // Verify password
        let argon2 = Argon2::default();
        let parsed_hash = PasswordHash::new(&user.password_hash)?;
        argon2.verify_password(password.as_bytes(), &parsed_hash)?;
        
        // Generate JWT
        let access_token = self.generate_jwt(&user)?;
        let refresh_token = self.generate_refresh_token(&user).await?;
        
        // Update last login
        self.user_storage.update_last_login(&user.user_id).await?;
        
        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_expiry_hours * 3600,
            user: UserInfo {
                user_id: user.user_id,
                username: user.username,
                role: user.role,
            },
        })
    }
    
    fn generate_jwt(&self, user: &User) -> Result<String, Error> {
        let now = Utc::now().timestamp() as usize;
        let exp = (Utc::now() + Duration::hours(self.jwt_expiry_hours)).timestamp() as usize;
        
        let claims = Claims {
            sub: user.user_id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            exp,
            iat: now,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;
        
        Ok(token)
    }
    
    pub fn verify_jwt(&self, token: &str) -> Result<Claims, Error> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )?;
        
        Ok(token_data.claims)
    }
}

// Axum extractor for JWT authentication
pub struct AuthUser(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized("Missing authorization header".to_string()))?;
        
        // Parse Bearer token
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized("Invalid authorization header".to_string()))?;
        
        // Verify token
        let claims = parts
            .extensions
            .get::<Arc<AuthService>>()
            .ok_or(ApiError::Internal("Auth service not available".to_string()))?
            .verify_jwt(token)?;
        
        Ok(AuthUser(claims))
    }
}

// API Key authentication
pub struct ApiKeyAuth(pub ApiKey);

#[async_trait]
impl<S> FromRequestParts<S> for ApiKeyAuth
where
    S: Send + Sync,
{
    type Rejection = ApiError;
    
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Try X-API-Key header
        let api_key = parts
            .headers
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized("Missing API key".to_string()))?;
        
        // Verify API key
        let key_record = parts
            .extensions
            .get::<Arc<ApiKeyService>>()
            .ok_or(ApiError::Internal("API key service not available".to_string()))?
            .verify_key(api_key)
            .await?;
        
        Ok(ApiKeyAuth(key_record))
    }
}
```

---

## Role-Based Access Control (RBAC)

### Roles

| Role | Description | Permissions |
|------|-------------|-------------|
| **admin** | Full system access | All operations |
| **user** | Regular user | Create/read/update own resources |
| **readonly** | Read-only access | Read operations only |

### Permission Checker

```rust
pub fn require_role(required_role: &str) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::any()
        .and(warp::ext::get::<Claims>())
        .and_then(move |claims: Claims| async move {
            if has_permission(&claims.role, required_role) {
                Ok(())
            } else {
                Err(warp::reject::custom(ForbiddenError))
            }
        })
        .untuple_one()
}

fn has_permission(user_role: &str, required_role: &str) -> bool {
    match required_role {
        "admin" => user_role == "admin",
        "user" => user_role == "admin" || user_role == "user",
        "readonly" => true,  // All roles can read
        _ => false,
    }
}

// Usage in routes
#[utoipa::path(
    delete,
    path = "/api/v1/documents/clear",
    tag = "Documents",
    security(
        ("bearer" = []),
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Documents cleared"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn clear_documents(
    State(state): State<AppState>,
    auth: AuthUser,  // JWT or API Key authenticated
) -> ApiResult<Json<ClearResponse>> {
    // Check permission
    if auth.0.role != "admin" {
        return Err(ApiError::Forbidden("Admin role required".to_string()));
    }
    
    // Clear documents
    // ...
}
```

---

## Configuration

```bash
# JWT Configuration
JWT_SECRET=your-256-bit-secret-key-here
JWT_EXPIRY_HOURS=24
REFRESH_TOKEN_EXPIRY_DAYS=30

# API Key Configuration
API_KEY_PREFIX=sk_
API_KEY_LENGTH=32

# Security
BCRYPT_COST=12
SESSION_TIMEOUT_MINUTES=30
MAX_LOGIN_ATTEMPTS=5
LOCKOUT_DURATION_MINUTES=15
```

---

**Status:** ✅ Specification Complete  
**Dependencies:** User storage, JWT library  
**Next:** 06-multi-tenancy.md
