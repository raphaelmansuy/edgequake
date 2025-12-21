# Security Implementation Guide for EdgeQuake

**Technology Stack**: Rust + Security Best Practices  
**Date**: 2025-12-21  
**Status**: Complete  
**Related**: [deployment-guide.md](./deployment-guide.md), [multi-tenancy-guide.md](./multi-tenancy-guide.md)

---

## Overview

This guide provides comprehensive security implementation patterns for EdgeQuake (Rust-based RAG system), covering authentication, authorization, input validation, rate limiting, API security, and threat mitigation.

**Security Principles**:

- Defense in depth (multiple security layers)
- Principle of least privilege
- Fail securely
- Security by design
- Assume breach mentality

---

## Threat Model

### Assets to Protect

1. **Data Assets**:
   - User documents and content
   - Knowledge graph entities and relations
   - API keys and credentials
   - Workspace data (multi-tenant isolation)
   - Vector embeddings

2. **System Assets**:
   - API endpoints
   - Database connections
   - LLM API credentials
   - Server infrastructure

### Threat Categories

| Threat | Risk | Impact | Mitigation |
|--------|------|--------|------------|
| Unauthorized access | High | Data breach | Authentication, Authorization |
| SQL/Query injection | High | Data corruption | Input validation, Parameterized queries |
| Cross-tenant data leakage | Critical | Privacy violation | Workspace isolation, Query filtering |
| API key exposure | High | Unauthorized usage | Secrets management, Encryption |
| DDoS attacks | Medium | Service disruption | Rate limiting, Load balancing |
| Prompt injection | Medium | LLM abuse | Input sanitization, Output validation |
| Man-in-the-middle | High | Data interception | TLS/SSL encryption |

---

## Authentication

### API Key Authentication

```rust
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Sha256, Digest};
use std::sync::Arc;

/// API key structure
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key_id: String,
    pub key_hash: String,  // SHA-256 hash of the key
    pub user_id: String,
    pub workspace_id: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl ApiKey {
    /// Generate new API key
    pub fn generate(user_id: String, workspace_id: Option<String>) -> (String, ApiKey) {
        let raw_key = format!(
            "edgequake_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        
        let key_hash = Self::hash_key(&raw_key);
        
        let api_key = ApiKey {
            key_id: uuid::Uuid::new_v4().to_string(),
            key_hash,
            user_id,
            workspace_id,
            scopes: vec!["read".to_string(), "write".to_string()],
            created_at: chrono::Utc::now(),
            expires_at: None,
            is_active: true,
        };
        
        (raw_key, api_key)
    }
    
    /// Hash API key using SHA-256
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Verify API key
    pub fn verify(&self, provided_key: &str) -> bool {
        if !self.is_active {
            return false;
        }
        
        if let Some(expires_at) = self.expires_at {
            if chrono::Utc::now() > expires_at {
                return false;
            }
        }
        
        let provided_hash = Self::hash_key(provided_key);
        provided_hash == self.key_hash
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract API key from header
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Validate API key
    let key_data = state.storage
        .get_api_key_by_value(api_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    if !key_data.verify(api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Store authenticated user in request extensions
    request.extensions_mut().insert(key_data.user_id.clone());
    if let Some(workspace_id) = &key_data.workspace_id {
        request.extensions_mut().insert(workspace_id.clone());
    }
    
    Ok(next.run(request).await)
}
```

### JWT Token Authentication (Optional)

```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub workspace_id: Option<String>,
    pub exp: usize,   // Expiration timestamp
    pub iat: usize,   // Issued at timestamp
    pub scopes: Vec<String>,
}

pub struct JWTAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JWTAuth {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
    
    pub fn generate_token(&self, user_id: String, workspace_id: Option<String>) -> Result<String, jsonwebtoken::errors::Error> {
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .unwrap()
            .timestamp() as usize;
        
        let claims = Claims {
            sub: user_id,
            workspace_id,
            exp: expiration,
            iat: chrono::Utc::now().timestamp() as usize,
            scopes: vec!["read".to_string(), "write".to_string()],
        };
        
        encode(&Header::default(), &claims, &self.encoding_key)
    }
    
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::default(),
        )?;
        
        Ok(token_data.claims)
    }
}
```

---

## Authorization

### Role-Based Access Control (RBAC)

```rust
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    ReadWorkspace,
    WriteWorkspace,
    DeleteWorkspace,
    ManageUsers,
    ManageApiKeys,
}

#[derive(Debug, Clone)]
pub enum Role {
    Admin,
    Member,
    Viewer,
}

impl Role {
    pub fn permissions(&self) -> HashSet<Permission> {
        match self {
            Role::Admin => {
                vec![
                    Permission::ReadWorkspace,
                    Permission::WriteWorkspace,
                    Permission::DeleteWorkspace,
                    Permission::ManageUsers,
                    Permission::ManageApiKeys,
                ]
                .into_iter()
                .collect()
            }
            Role::Member => {
                vec![
                    Permission::ReadWorkspace,
                    Permission::WriteWorkspace,
                ]
                .into_iter()
                .collect()
            }
            Role::Viewer => {
                vec![Permission::ReadWorkspace]
                    .into_iter()
                    .collect()
            }
        }
    }
    
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions().contains(permission)
    }
}

/// Authorization middleware
pub async fn require_permission(
    permission: Permission,
) -> impl Fn(
    State<Arc<AppState>>,
    Request,
    Next,
) -> impl std::future::Future<Output = Result<Response, StatusCode>> {
    move |State(state): State<Arc<AppState>>,
          request: Request,
          next: Next| async move {
        // Get authenticated user from request extensions
        let user_id = request
            .extensions()
            .get::<String>()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        
        // Get user role
        let user = state.storage
            .get_user(user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        
        // Check permission
        if !user.role.has_permission(&permission) {
            return Err(StatusCode::FORBIDDEN);
        }
        
        Ok(next.run(request).await)
    }
}
```

---

## Input Validation

### Request Validation

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWorkspaceRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    #[validate(length(max = 500))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct InsertDocumentRequest {
    #[validate(length(min = 1, max = 1_000_000))]
    pub content: String,
    
    #[validate(custom = "validate_metadata")]
    pub metadata: Option<serde_json::Value>,
}

fn validate_metadata(metadata: &serde_json::Value) -> Result<(), ValidationError> {
    // Ensure metadata is an object
    if !metadata.is_object() {
        return Err(ValidationError::new("metadata_must_be_object"));
    }
    
    // Limit metadata size
    let serialized = serde_json::to_string(metadata).unwrap();
    if serialized.len() > 10_000 {
        return Err(ValidationError::new("metadata_too_large"));
    }
    
    Ok(())
}

/// Validation middleware
pub async fn validate_request<T: Validate>(
    Json(payload): Json<T>,
) -> Result<Json<T>, (StatusCode, String)> {
    payload
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    
    Ok(Json(payload))
}
```

### SQL Injection Prevention

```rust
// ✅ CORRECT - Use parameterized queries
pub async fn get_entity_safe(
    db: &Surreal<Client>,
    entity_id: &str,
    workspace_id: &str,
) -> Result<Option<Entity>> {
    let result: Option<Entity> = db
        .query("SELECT * FROM entity WHERE id = $id AND workspace_id = $workspace_id")
        .bind(("id", entity_id))
        .bind(("workspace_id", workspace_id))
        .await?
        .take(0)?;
    
    Ok(result)
}

// ❌ INCORRECT - Vulnerable to injection
pub async fn get_entity_unsafe(
    db: &Surreal<Client>,
    entity_id: &str,
    workspace_id: &str,
) -> Result<Option<Entity>> {
    // NEVER DO THIS!
    let query = format!(
        "SELECT * FROM entity WHERE id = '{}' AND workspace_id = '{}'",
        entity_id, workspace_id
    );
    
    let result: Option<Entity> = db
        .query(&query)
        .await?
        .take(0)?;
    
    Ok(result)
}
```

### Prompt Injection Prevention

```rust
pub struct PromptSanitizer;

impl PromptSanitizer {
    /// Sanitize user input to prevent prompt injection
    pub fn sanitize(input: &str) -> String {
        // Remove control characters
        let cleaned = input
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect::<String>();
        
        // Limit length
        let truncated = if cleaned.len() > 10_000 {
            &cleaned[..10_000]
        } else {
            &cleaned
        };
        
        // Escape special sequences that could break prompts
        truncated
            .replace("```", "'''")  // Prevent markdown code blocks
            .replace("---", "___")  // Prevent markdown separators
    }
    
    /// Validate LLM output
    pub fn validate_output(output: &str) -> Result<String, String> {
        // Check for suspicious patterns
        if output.contains("IGNORE PREVIOUS INSTRUCTIONS") {
            return Err("Suspicious output detected".to_string());
        }
        
        // Limit output length
        if output.len() > 50_000 {
            return Err("Output too long".to_string());
        }
        
        Ok(output.to_string())
    }
}
```

---

## Rate Limiting

### Token Bucket Rate Limiter

```rust
use tower_governor::{
    governor::GovernorConfigBuilder,
    key_extractor::KeyExtractor,
    GovernorLayer,
};
use std::net::SocketAddr;

/// Extract rate limit key from request
#[derive(Clone)]
pub struct ApiKeyExtractor;

impl KeyExtractor for ApiKeyExtractor {
    type Key = String;
    
    fn extract(&self, req: &Request<Body>) -> Result<Self::Key, tower_governor::errors::GovernorError> {
        // Extract API key from header
        let api_key = req
            .headers()
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string();
        
        Ok(api_key)
    }
}

/// Create rate limiter
pub fn create_rate_limiter() -> GovernorLayer<ApiKeyExtractor> {
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(10)  // 10 requests per second
            .burst_size(20)  // Allow burst of 20 requests
            .finish()
            .unwrap()
    );
    
    GovernorLayer {
        config: Box::leak(governor_conf),
    }
}

/// Apply to routes
pub fn create_app() -> Router {
    Router::new()
        .route("/query", post(query_handler))
        .layer(create_rate_limiter())
}
```

### Adaptive Rate Limiting

```rust
use std::collections::HashMap;
use std::sync::RwLock;

pub struct AdaptiveRateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimit>>>,
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub current_count: u32,
    pub window_start: chrono::DateTime<chrono::Utc>,
}

impl AdaptiveRateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn check_limit(&self, api_key: &str) -> bool {
        let mut limits = self.limits.write().unwrap();
        let now = chrono::Utc::now();
        
        let limit = limits.entry(api_key.to_string()).or_insert(RateLimit {
            requests_per_minute: 60,
            current_count: 0,
            window_start: now,
        });
        
        // Reset window if expired
        if now.signed_duration_since(limit.window_start).num_seconds() >= 60 {
            limit.current_count = 0;
            limit.window_start = now;
        }
        
        // Check limit
        if limit.current_count >= limit.requests_per_minute {
            return false;
        }
        
        limit.current_count += 1;
        true
    }
    
    pub fn adjust_limit(&self, api_key: &str, new_limit: u32) {
        let mut limits = self.limits.write().unwrap();
        if let Some(limit) = limits.get_mut(api_key) {
            limit.requests_per_minute = new_limit;
        }
    }
}
```

---

## Secrets Management

### Environment-Based Secrets

```rust
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Secrets {
    pub openai_api_key: String,
    pub surrealdb_password: String,
    pub jwt_secret: String,
}

impl Secrets {
    pub fn load() -> Result<Self, String> {
        dotenv().ok();
        
        let openai_api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| "Missing OPENAI_API_KEY".to_string())?;
        
        let surrealdb_password = env::var("SURREALDB_PASSWORD")
            .map_err(|_| "Missing SURREALDB_PASSWORD".to_string())?;
        
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "Missing JWT_SECRET".to_string())?;
        
        // Validate secrets
        if openai_api_key.is_empty() {
            return Err("OPENAI_API_KEY cannot be empty".to_string());
        }
        
        Ok(Self {
            openai_api_key,
            surrealdb_password,
            jwt_secret,
        })
    }
}
```

### Encrypted Storage

```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{Engine as _, engine::general_purpose};

pub struct EncryptedStorage {
    cipher: Aes256Gcm,
}

impl EncryptedStorage {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }
    
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let nonce = Nonce::from_slice(b"unique nonce"); // Use proper random nonce
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| format!("Encryption failed: {}", e))?;
        
        Ok(general_purpose::STANDARD.encode(ciphertext))
    }
    
    pub fn decrypt(&self, ciphertext: &str) -> Result<String, String> {
        let nonce = Nonce::from_slice(b"unique nonce"); // Must match encryption nonce
        
        let decoded = general_purpose::STANDARD
            .decode(ciphertext)
            .map_err(|e| format!("Base64 decode failed: {}", e))?;
        
        let plaintext = self.cipher
            .decrypt(nonce, decoded.as_ref())
            .map_err(|e| format!("Decryption failed: {}", e))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| format!("UTF-8 decode failed: {}", e))
    }
}
```

---

## TLS/SSL Configuration

### Axum with TLS

```rust
use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;

pub async fn run_with_tls(app: Router, addr: SocketAddr) -> Result<()> {
    // Load TLS certificates
    let config = RustlsConfig::from_pem_file(
        "certs/cert.pem",
        "certs/key.pem"
    )
    .await?;
    
    // Run server with TLS
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

---

## Audit Logging

### Security Event Logger

```rust
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: SecurityEventType,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub enum SecurityEventType {
    AuthenticationSuccess,
    AuthenticationFailure,
    AuthorizationFailure,
    RateLimitExceeded,
    SuspiciousActivity,
    DataAccess,
    DataModification,
}

pub struct SecurityLogger {
    // Could be file, database, or external service
}

impl SecurityLogger {
    pub async fn log_event(&self, event: SecurityEvent) {
        // Log to appropriate destination
        let log_line = serde_json::to_string(&event).unwrap();
        eprintln!("[SECURITY] {}", log_line);
        
        // Could also send to SIEM, Splunk, etc.
    }
    
    pub async fn log_auth_failure(&self, user_id: Option<String>, ip: Option<String>) {
        self.log_event(SecurityEvent {
            timestamp: chrono::Utc::now(),
            event_type: SecurityEventType::AuthenticationFailure,
            user_id,
            ip_address: ip,
            details: serde_json::json!({}),
        }).await;
    }
}
```

---

## Security Testing

### Security Test Cases

```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_workspace_isolation() {
        let app = test_app().await;
        
        // Create two workspaces
        let ws1 = create_test_workspace(&app, "Workspace 1").await;
        let ws2 = create_test_workspace(&app, "Workspace 2").await;
        
        // Insert data in workspace 1
        insert_test_data(&app, &ws1.id, "secret data").await;
        
        // Try to access from workspace 2
        let response = app
            .get(&format!("/workspace/{}/data", ws2.id))
            .send()
            .await
            .unwrap();
        
        // Should not see workspace 1 data
        assert_eq!(response.status(), 404);
    }
    
    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let app = test_app().await;
        
        let malicious_input = "'; DROP TABLE entity; --";
        
        let response = app
            .post("/query")
            .json(&json!({"query": malicious_input}))
            .send()
            .await
            .unwrap();
        
        // Should handle safely
        assert!(response.status().is_success() || response.status() == 400);
        
        // Verify table still exists
        let entities = app.get_entities().await.unwrap();
        assert!(!entities.is_empty());
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let app = test_app().await;
        
        // Send requests exceeding rate limit
        for _ in 0..100 {
            let response = app.post("/query").json(&json!({"query": "test"})).send().await.unwrap();
            if response.status() == 429 {
                // Rate limit triggered
                return;
            }
        }
        
        panic!("Rate limit not enforced");
    }
}
```

---

## Security Checklist

### Pre-Production

- [ ] All secrets stored in environment variables
- [ ] TLS/SSL enabled for all connections
- [ ] API authentication implemented
- [ ] Authorization checks on all endpoints
- [ ] Input validation on all user inputs
- [ ] Rate limiting configured
- [ ] Audit logging enabled
- [ ] Workspace isolation tested
- [ ] SQL injection prevention verified
- [ ] Prompt injection mitigation implemented

### Production

- [ ] Security headers configured (HSTS, CSP, etc.)
- [ ] DDoS protection enabled
- [ ] Backup encryption enabled
- [ ] Monitoring and alerting configured
- [ ] Incident response plan documented
- [ ] Regular security audits scheduled
- [ ] Dependency vulnerability scanning enabled
- [ ] Penetration testing completed

---

## Conclusion

This security guide provides comprehensive implementation patterns for securing the EdgeQuake application. Security must be implemented at every layer, from authentication to data storage.

**Key Takeaways**:

1. Use strong authentication (API keys + JWT)
2. Implement fine-grained authorization (RBAC)
3. Validate all inputs thoroughly
4. Rate limit API requests
5. Encrypt sensitive data at rest and in transit
6. Log all security events
7. Test security continuously

**Next Steps**:

- Implement security middleware
- Set up audit logging
- Configure rate limiting
- Enable TLS/SSL
- Conduct security testing

---

**Status**: ✅ COMPLETE - Security guide ready for implementation
