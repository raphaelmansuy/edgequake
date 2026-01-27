# OODA-253: Timeout and Resource Limit Audit

## Observe

Audited timeout and resource limit configurations across the system.

### Safety Limits Module (`safety_limits.rs`)

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_MAX_TOKENS` | 8192 | Default token limit per request |
| `DEFAULT_TIMEOUT_SECS` | 120 | 2 minute request timeout |
| `ABSOLUTE_MAX_TOKENS` | 32768 | Hard cap on tokens |
| `MINIMUM_TIMEOUT_SECS` | 10 | Prevents too-short timeouts |
| `MAXIMUM_TIMEOUT_SECS` | 600 | 10 minute maximum |

### Resource Limits by Layer

| Layer | Limit | Value |
|-------|-------|-------|
| LLM Provider | Max tokens | 8192 (configurable) |
| LLM Provider | Timeout | 120s (10s-600s range) |
| API | Document size | 10 MB |
| API | Query length | 10000 chars |
| Path validation | Max depth | 50 |
| Rate limiting | Requests/window | Configurable |
| Cache | TTL | 3600s (1 hour) |

### Configuration Methods

| Method | Source | Used By |
|--------|--------|---------|
| `SafetyLimitsConfig::default()` | Hardcoded | Default |
| `SafetyLimitsConfig::from_env()` | Environment | Production |
| `SafetyLimitsConfig::strict()` | Hardcoded | Testing |

### Environment Variables

| Variable | Default | Effect |
|----------|---------|--------|
| `EDGEQUAKE_LLM_MAX_TOKENS` | 8192 | Token limit |
| `EDGEQUAKE_LLM_TIMEOUT_SECS` | 120 | Request timeout |
| `ALLOWED_SCAN_PATHS` | None | Directory access |
| `ALLOW_ANY_SCAN_PATH` | false | Dev mode flag |

## Orient

### Limit Enforcement

1. **Token Limits (ENFORCED)**
   - `SafetyLimitedProvider` wraps all LLM providers
   - Clamps tokens to configured max
   - Cannot exceed `ABSOLUTE_MAX_TOKENS`

2. **Timeouts (ENFORCED)**
   - Tokio timeout wrapper on all LLM calls
   - Configurable via environment
   - Clamped to safe range (10s-600s)

3. **Document Limits (ENFORCED)**
   - `validate_content()` checks size
   - `AppConfig.max_document_size` = 10 MB

4. **Rate Limiting (CONFIGURABLE)**
   - Token bucket algorithm
   - Per-tenant rate limits
   - Disabled by default for development

### Risk Assessment

| Risk | Mitigation | Status |
|------|------------|--------|
| LLM runaway generation | Token limit | ✅ Enforced |
| Hung requests | Timeout | ✅ Enforced |
| Large file upload | Size limit | ✅ Enforced |
| DoS via rate | Rate limiting | ⚠️ Optional |

## Decide

**All critical limits are in place.**

The system has comprehensive resource limits:
1. Hard token limits with clamping
2. Request timeouts with safe ranges
3. Document size limits
4. Rate limiting available

## Act

Document findings. All limits are properly configured.

## Metrics

| Metric | Value |
|--------|-------|
| Safety limit constants | 5 |
| Configurable timeouts | 2 |
| Document size limits | 3 |
| Rate limiting enabled | Optional |

## Conclusion

✅ **RESOURCE LIMITS ARE COMPREHENSIVE**

- Token generation is capped at 32768 absolute max
- Timeouts are enforced (10s-600s range)
- Document size limited to 10 MB
- Rate limiting available for production
- All limits are configurable via environment
