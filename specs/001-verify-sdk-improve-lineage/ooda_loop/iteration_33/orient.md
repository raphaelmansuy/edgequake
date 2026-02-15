# OODA-33 Orient: PHP SDK Enhancement Strategy

## Analysis

### Root Cause of Low Coverage

1. PHP SDK was created as minimal MVP with basic CRUD
2. Missing HTTP verbs (PUT/PATCH) blocked update operations
3. No auth, workspace, or shared conversation support
4. Many essential endpoints not exposed

### First Principles Assessment

- **User expectation**: Complete API coverage for production use
- **Developer productivity**: Missing methods force raw HTTP workarounds
- **Feature parity**: PHP developers deserve same APIs as Python/TypeScript

### Risk Analysis

| Approach               | Risk                | Mitigation                                |
| ---------------------- | ------------------- | ----------------------------------------- |
| Bulk addition          | Breaking changes    | Keep existing method signatures identical |
| New services           | Namespace pollution | Use same patterns as existing services    |
| MockHttpHelper changes | Test breakage       | Add methods, don't modify existing        |

### Priority Matrix

```
                    HIGH IMPACT
                        │
    AuthService    ─────┼───── WorkspaceService
    (security)          │      (multi-tenant)
                        │
LOW EFFORT ─────────────┼───────────── HIGH EFFORT
                        │
    HealthService  ─────┼───── ConversationService
    (+3 methods)        │      (+10 methods)
                        │
                   LOW IMPACT
```

### Decision Factors

1. Follow established PHP SDK patterns (single Services.php file)
2. Use rawurlencode() for URL parameters
3. Support optional parameters with null defaults
4. Return arrays consistently (no custom DTOs)

## Recommended Approach

Add all missing endpoints in one iteration to maximize impact and reach 95%+ coverage. The PHP SDK file is compact (~280 lines) so bulk updates are manageable.
