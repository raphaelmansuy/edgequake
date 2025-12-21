# EdgeQuake Configuration Management Guide

**Version**: 1.0  
**Date**: 2024-12-21  
**Audience**: Operators, DevOps Engineers

---

## Table of Contents

1. [Overview](#overview)
2. [Configuration Sources](#configuration-sources)
3. [Environment Variables](#environment-variables)
4. [Configuration Files](#configuration-files)
5. [Runtime Configuration](#runtime-configuration)
6. [Secrets Management](#secrets-management)
7. [Feature Flags](#feature-flags)
8. [Configuration Validation](#configuration-validation)

---

## Overview

EdgeQuake supports multiple configuration sources, prioritized as follows:

```
CLI Arguments (highest priority)
    ↓
Environment Variables
    ↓
Configuration Files (TOML/YAML)
    ↓
Defaults (lowest priority)
```

**Configuration Philosophy**:
- **Environment variables** for deployment-specific values (URLs, secrets)
- **Configuration files** for application logic (timeouts, limits, features)
- **CLI arguments** for one-off overrides (development, debugging)
- **Defaults** embedded in code (safe fallbacks)

---

## Configuration Sources

### 1. Environment Variables

**Priority**: High (overrides files, but not CLI args)  
**Use Case**: Deployment-specific configuration (URLs, secrets)  
**Format**: `EDGEQUAKE_*` prefix (e.g., `EDGEQUAKE_DATABASE_URL`)

**Example**:
```bash
export EDGEQUAKE_DATABASE_URL="postgresql://user:pass@localhost/db"
export EDGEQUAKE_LLM_API_KEY="sk-..."
export EDGEQUAKE_LOG_LEVEL="debug"
```

---

### 2. Configuration Files

**Priority**: Medium (overridden by env vars and CLI args)  
**Use Case**: Application-wide settings (timeouts, limits, features)  
**Formats**: TOML (preferred), YAML, JSON

**Example (`config.toml`)**:
```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgresql://localhost/edgequake"
pool_size = 10
connection_timeout = 30

[llm]
provider = "openai"
model = "gpt-4"
max_tokens = 2000
temperature = 0.7

[query]
default_mode = "hybrid"
max_query_length = 1000
timeout = 60

[features]
multi_tenancy = true
rate_limiting = true
caching = false
```

---

### 3. CLI Arguments

**Priority**: Highest (overrides all other sources)  
**Use Case**: One-off testing, debugging  
**Format**: `--flag value` or `--flag=value`

**Example**:
```bash
edgequake \
  --host 0.0.0.0 \
  --port 8080 \
  --config /path/to/config.toml \
  --log-level debug \
  --database-url postgresql://localhost/edgequake
```

---

### 4. Defaults

**Priority**: Lowest (used when no other source provides value)  
**Use Case**: Safe fallbacks for development  
**Location**: Embedded in code

**Example (Rust)**:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    
    #[serde(default = "default_port")]
    pub port: u16,
    
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}
```

---

## Environment Variables

### Core Configuration

| Variable | Description | Example | Default |
|----------|-------------|---------|---------|
| `EDGEQUAKE_HOST` | Server bind address | `0.0.0.0` | `127.0.0.1` |
| `EDGEQUAKE_PORT` | Server port | `8080` | `8080` |
| `EDGEQUAKE_WORKERS` | Worker threads | `4` | CPU count |
| `EDGEQUAKE_LOG_LEVEL` | Logging level | `debug`, `info`, `warn`, `error` | `info` |
| `EDGEQUAKE_CONFIG_FILE` | Path to config file | `/etc/edgequake/config.toml` | `./config.toml` |

### Database Configuration

| Variable | Description | Example | Default |
|----------|-------------|---------|---------|
| `EDGEQUAKE_DATABASE_URL` | PostgreSQL database URL | `postgresql://localhost/edgequake` | *Required* |
| `EDGEQUAKE_DATABASE_POOL_SIZE` | Connection pool size | `10` | `10` |
| `EDGEQUAKE_DATABASE_TIMEOUT` | Connection timeout (sec) | `30` | `30` |
| `EDGEQUAKE_ENABLE_AGE` | Enable Apache AGE extension | `true`, `false` | `true` |
| `EDGEQUAKE_ENABLE_PGVECTOR` | Enable pgvector extension | `true`, `false` | `true` |
| `EDGEQUAKE_REDIS_URL` | Redis URL (caching) | `redis://localhost:6379` | *Optional* |

### LLM Configuration

| Variable | Description | Example | Default |
|----------|-------------|---------|---------|
| `EDGEQUAKE_LLM_PROVIDER` | LLM provider | `openai`, `anthropic`, `local` | `openai` |
| `EDGEQUAKE_LLM_API_KEY` | LLM API key | `sk-...` | *Required* |
| `EDGEQUAKE_LLM_MODEL` | Model name | `gpt-4`, `gpt-3.5-turbo` | `gpt-3.5-turbo` |
| `EDGEQUAKE_LLM_MAX_TOKENS` | Max response tokens | `2000` | `1000` |
| `EDGEQUAKE_LLM_TEMPERATURE` | Sampling temperature | `0.7` | `0.0` |
| `EDGEQUAKE_LLM_TIMEOUT` | Request timeout (sec) | `60` | `30` |

### Security Configuration

| Variable | Description | Example | Default |
|----------|-------------|---------|---------|
| `EDGEQUAKE_JWT_SECRET` | JWT signing secret | `base64-encoded-secret` | *Required* |
| `EDGEQUAKE_JWT_EXPIRY` | JWT expiry (seconds) | `3600` | `3600` |
| `EDGEQUAKE_API_KEY_SALT` | API key hashing salt | `random-salt` | *Generated* |
| `EDGEQUAKE_CORS_ORIGINS` | Allowed CORS origins | `https://app.example.com` | `*` |
| `EDGEQUAKE_RATE_LIMIT` | Global rate limit (req/sec) | `100` | `100` |

### Feature Flags

| Variable | Description | Example | Default |
|----------|-------------|---------|---------|
| `EDGEQUAKE_ENABLE_MULTI_TENANCY` | Enable multi-tenancy | `true`, `false` | `true` |
| `EDGEQUAKE_ENABLE_CACHING` | Enable Redis caching | `true`, `false` | `false` |
| `EDGEQUAKE_ENABLE_METRICS` | Enable Prometheus metrics | `true`, `false` | `true` |
| `EDGEQUAKE_ENABLE_TRACING` | Enable distributed tracing | `true`, `false` | `false` |

---

## Configuration Files

### TOML Format (Preferred)

**File**: `config.toml`

```toml
# Server Configuration
[server]
host = "0.0.0.0"
port = 8080
workers = 4
shutdown_timeout = 30  # seconds

# Database Configuration
[database]
url = "postgresql://edgequake:password@localhost/edgequake_prod"
pool_size = 10
connection_timeout = 30
idle_timeout = 600
max_lifetime = 1800

[database.surreal]
enabled = true
url = "ws://localhost:8000/rpc"
namespace = "prod"
database = "edgequake"

# LLM Configuration
[llm]
provider = "openai"
api_key_env = "EDGEQUAKE_LLM_API_KEY"  # Load from env var
model = "gpt-4"
max_tokens = 2000
temperature = 0.7
timeout = 60
retry_attempts = 3
retry_delay = 1  # seconds

[llm.embeddings]
model = "text-embedding-ada-002"
dimensions = 1536
batch_size = 100

# Query Configuration
[query]
default_mode = "hybrid"
max_query_length = 1000
timeout = 60
max_entities_per_query = 100
max_graph_depth = 3

[query.modes]
naive_enabled = true
local_enabled = true
global_enabled = true
hybrid_enabled = true

# Security Configuration
[security]
jwt_secret_env = "EDGEQUAKE_JWT_SECRET"
jwt_expiry = 3600  # 1 hour
api_key_salt_env = "EDGEQUAKE_API_KEY_SALT"
cors_origins = ["https://app.example.com", "https://admin.example.com"]
rate_limit_global = 100  # requests per second
rate_limit_per_tenant = 10

# Caching Configuration
[cache]
enabled = false
redis_url_env = "EDGEQUAKE_REDIS_URL"
query_ttl = 3600  # 1 hour
community_ttl = 86400  # 24 hours
embedding_ttl = 0  # never expire

# Logging Configuration
[logging]
level = "info"
format = "json"  # or "pretty"
stdout = true
file = "/var/log/edgequake/edgequake.log"
max_size = "100MB"
max_backups = 7

# Monitoring Configuration
[monitoring]
metrics_enabled = true
metrics_port = 9090
tracing_enabled = false
tracing_endpoint = "http://jaeger:14268/api/traces"

# Feature Flags
[features]
multi_tenancy = true
rate_limiting = true
caching = false
streaming = false
batch_processing = true
```

---

### YAML Format (Alternative)

**File**: `config.yaml`

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  workers: 4

database:
  url: "postgresql://localhost/edgequake"
  pool_size: 10
  connection_timeout: 30

llm:
  provider: "openai"
  model: "gpt-4"
  max_tokens: 2000
  temperature: 0.7

query:
  default_mode: "hybrid"
  max_query_length: 1000
  timeout: 60

features:
  multi_tenancy: true
  rate_limiting: true
  caching: false
```

---

### JSON Format (Alternative)

**File**: `config.json`

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "workers": 4
  },
  "database": {
    "url": "postgresql://localhost/edgequake",
    "pool_size": 10,
    "connection_timeout": 30
  },
  "llm": {
    "provider": "openai",
    "model": "gpt-4",
    "max_tokens": 2000,
    "temperature": 0.7
  }
}
```

---

## Runtime Configuration

### Loading Configuration

**Priority Order**:
1. Load defaults from code
2. Load configuration file (if specified)
3. Override with environment variables
4. Override with CLI arguments

**Example (Rust)**:
```rust
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub llm: LLMConfig,
    pub query: QueryConfig,
    pub features: FeatureFlags,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let mut cfg = Config::builder()
            // 1. Load defaults
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            
            // 2. Load config file (if exists)
            .add_source(File::with_name("config").required(false))
            
            // 3. Override with environment variables (EDGEQUAKE_* prefix)
            .add_source(
                Environment::with_prefix("EDGEQUAKE")
                    .separator("__")  // EDGEQUAKE__SERVER__PORT = server.port
            )
            
            .build()?;
        
        cfg.try_deserialize()
    }
}

// Usage
fn main() {
    let config = AppConfig::load().expect("Failed to load configuration");
    println!("Server will listen on {}:{}", config.server.host, config.server.port);
}
```

---

### Dynamic Configuration Reload

**Use Case**: Update configuration without restarting server

**Implementation**:
```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct ConfigWatcher {
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigWatcher {
    pub fn new(config_path: &str) -> Self {
        let config = Arc::new(RwLock::new(AppConfig::load().unwrap()));
        let config_clone = Arc::clone(&config);
        
        // Watch config file for changes
        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
        watcher.watch(config_path, RecursiveMode::NonRecursive).unwrap();
        
        // Reload config on file change
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if let Ok(new_config) = AppConfig::load() {
                    *config_clone.write().unwrap() = new_config;
                    info!("Configuration reloaded");
                }
            }
        });
        
        Self { config }
    }
    
    pub fn get(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }
}
```

---

## Secrets Management

### DO NOT Store Secrets in Config Files

**Bad** ❌:
```toml
[llm]
api_key = "sk-abc123..."  # NEVER do this!

[database]
password = "mysecretpass"  # NEVER do this!
```

**Good** ✅:
```toml
[llm]
api_key_env = "EDGEQUAKE_LLM_API_KEY"  # Reference env var

[database]
url_env = "EDGEQUAKE_DATABASE_URL"  # Reference env var
```

---

### Environment Variable Secrets

**Development** (`.env` file):
```bash
# .env (DO NOT COMMIT)
EDGEQUAKE_LLM_API_KEY=sk-abc123...
EDGEQUAKE_JWT_SECRET=$(openssl rand -base64 32)
EDGEQUAKE_DATABASE_URL=postgresql://user:pass@localhost/db
```

**Production** (Kubernetes Secret):
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: edgequake-secrets
type: Opaque
stringData:
  llm-api-key: sk-abc123...
  jwt-secret: base64-encoded-secret
  database-url: postgresql://user:pass@host/db
```

**Deploy to Pod**:
```yaml
env:
  - name: EDGEQUAKE_LLM_API_KEY
    valueFrom:
      secretKeyRef:
        name: edgequake-secrets
        key: llm-api-key
```

---

### Vault Integration (Advanced)

**HashiCorp Vault**:
```rust
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

async fn load_secrets() -> Result<Secrets, Error> {
    let client = VaultClient::new(
        VaultClientSettingsBuilder::default()
            .address("https://vault.example.com")
            .token(&std::env::var("VAULT_TOKEN")?)
            .build()?
    )?;
    
    let llm_key: String = client.kv2::read("secret/edgequake/llm_api_key").await?;
    let jwt_secret: String = client.kv2::read("secret/edgequake/jwt_secret").await?;
    
    Ok(Secrets { llm_key, jwt_secret })
}
```

---

## Feature Flags

### Boolean Flags

**Config**:
```toml
[features]
multi_tenancy = true
caching = false
streaming = false
```

**Usage**:
```rust
if config.features.multi_tenancy {
    // Enable tenant isolation middleware
    app = app.layer(tenant_middleware());
}

if config.features.caching {
    // Enable Redis caching
    app = app.layer(Extension(redis_client));
}
```

---

### Percentage Rollout Flags

**Config**:
```toml
[features.rollout]
new_query_algorithm = 10  # 10% of users

[features.rollout.beta_users]
enabled_for = ["user123", "user456"]
```

**Usage**:
```rust
fn should_use_new_algorithm(user_id: &str, config: &Config) -> bool {
    // Check if user is in beta list
    if config.features.rollout.beta_users.contains(user_id) {
        return true;
    }
    
    // Otherwise, use percentage rollout
    let hash = hash(user_id) % 100;
    hash < config.features.rollout.new_query_algorithm
}
```

---

## Configuration Validation

### Startup Validation

**Validate on startup** (fail-fast):
```rust
impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate port range
        if self.server.port < 1024 || self.server.port > 65535 {
            return Err(ConfigError::Message(
                format!("Invalid port: {}", self.server.port)
            ));
        }
        
        // Validate database URL
        if !self.database.url.starts_with("postgresql://") {
            return Err(ConfigError::Message(
                "Database URL must start with postgresql://"
            ));
        }
        
        // Validate LLM API key format
        if !self.llm.api_key.starts_with("sk-") {
            return Err(ConfigError::Message(
                "Invalid OpenAI API key format"
            ));
        }
        
        // Validate timeouts
        if self.query.timeout == 0 {
            return Err(ConfigError::Message(
                "Query timeout must be > 0"
            ));
        }
        
        Ok(())
    }
}

// Usage
fn main() {
    let config = AppConfig::load().expect("Failed to load config");
    config.validate().expect("Invalid configuration");
    
    // Start server...
}
```

---

### Schema Validation

**Use serde validation**:
```rust
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ServerConfig {
    #[validate(range(min = 1024, max = 65535))]
    pub port: u16,
    
    #[validate(range(min = 1, max = 128))]
    pub workers: usize,
    
    #[validate(url)]
    pub base_url: String,
}

// Validate during deserialization
let config: ServerConfig = toml::from_str(toml_str)?;
config.validate()?;  // Returns error if invalid
```

---

## Best Practices

### 1. Separate Concerns

- **Secrets** → Environment variables / Vault
- **Infrastructure** → Environment variables (URLs, ports)
- **Application Logic** → Configuration files (timeouts, limits)
- **Feature Flags** → Configuration files + dynamic toggles

### 2. Fail Fast

- Validate configuration at startup
- Don't wait for runtime errors
- Provide clear error messages

### 3. Document Defaults

- Every config option should have a sensible default
- Document defaults in code and README

### 4. Environment-Specific Files

```
config/
├── default.toml       # Shared defaults
├── development.toml   # Dev-specific overrides
├── staging.toml       # Staging-specific overrides
└── production.toml    # Production-specific overrides
```

Load with:
```rust
let env = std::env::var("EDGEQUAKE_ENV").unwrap_or("development".to_string());
let config = Config::builder()
    .add_source(File::with_name("config/default"))
    .add_source(File::with_name(&format!("config/{}", env)))
    .build()?;
```

### 5. Version Configuration Files

- Track configuration in git
- Use `.env.example` for secrets (not `.env`)
- Document breaking changes in CHANGELOG

---

## Troubleshooting

### Config File Not Found

**Error**: `Config file 'config.toml' not found`

**Solution**:
```bash
# Specify path with CLI
edgequake --config /path/to/config.toml

# Or set environment variable
export EDGEQUAKE_CONFIG_FILE=/path/to/config.toml
```

### Invalid Environment Variable Format

**Error**: `Failed to parse EDGEQUAKE__SERVER__PORT: invalid digit`

**Solution**:
- Use double underscore `__` as separator: `EDGEQUAKE__SERVER__PORT`
- Ensure value is correct type: `export EDGEQUAKE__SERVER__PORT=8080`

### Secret Not Loaded

**Error**: `LLM API key not found`

**Solution**:
```bash
# Check env var is set
echo $EDGEQUAKE_LLM_API_KEY

# If empty, export it
export EDGEQUAKE_LLM_API_KEY=sk-...

# Or add to .env file
echo "EDGEQUAKE_LLM_API_KEY=sk-..." >> .env
```

---

## References

- **Rust config crate**: https://docs.rs/config/
- **12-Factor App**: https://12factor.net/config
- **Kubernetes Secrets**: https://kubernetes.io/docs/concepts/configuration/secret/
- **HashiCorp Vault**: https://www.vaultproject.io/docs

---

**Next Steps**:
- Review [security-guide.md](./security-guide.md) for secrets management
- See [deployment-guide.md](./deployment-guide.md) for production configuration
- Check [DEVELOPER_QUICKSTART.md](../integration/DEVELOPER_QUICKSTART.md) for local setup
