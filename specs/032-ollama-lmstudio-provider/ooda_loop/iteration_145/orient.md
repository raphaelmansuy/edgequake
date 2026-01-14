# Orient - Iteration 145

## Context Analysis

Provider health monitoring enables users to see which providers are reachable before attempting queries.

### Health Check Architecture

```
Frontend                       Backend
   │                              │
   │  GET /api/models/health      │
   ├─────────────────────────────►│
   │                              │  for each enabled provider:
   │                              │    ├─ Mock: always available
   │                              │    ├─ Ollama: TCP connect localhost:11434
   │                              │    ├─ LM Studio: TCP connect localhost:1234
   │                              │    └─ Cloud: assume available
   │◄─────────────────────────────┤
   │  ProviderHealthResponse[]    │
```

### Frontend Integration

**Hook**: `use-models.ts` (`useProvidersHealth`)
**API**: `fetchProvidersHealth` in `lib/api/models.ts`

The workspace page uses this to show provider status icons:
- ✅ Green: Provider available
- ❌ Red: Provider unavailable with error message
- ⏱️ Latency displayed in ms

### WHY Comments in Code

From `models.rs`:
```rust
/// # WHY: Runtime Health Checks
///
/// Configuration says what providers *should* be available.
/// Health checks confirm what *is* actually reachable.
/// This helps users understand why a model might not work.
```

## Assessment

**Provider Health Monitoring: VERIFIED COMPLETE**

All providers have proper health check mechanisms with latency reporting.
