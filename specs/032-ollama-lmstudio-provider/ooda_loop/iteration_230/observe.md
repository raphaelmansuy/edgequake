# OODA Loop Iteration 230 - Security Test Layer

## OBSERVE

### Objective

Create an "inviolable security test layer" that enforces critical invariants at compile/test time.

### Critical Invariants to Enforce

1. **SAFE_PROVIDER_CREATION**: All production provider creation uses safe variants
2. **TENANT_ISOLATION**: Query operations use workspace's tenant_id, not header tenant_id
3. **TIMEOUT_PROTECTION**: All external API calls have timeout limits
4. **ERROR_CLASSIFICATION**: API key errors return proper HTTP status codes

## ORIENT

### First-Principles Approach

A security test layer should:
1. **Be automatic**: Run on every CI build
2. **Be fast**: Not add significant build time
3. **Be comprehensive**: Cover all critical invariants
4. **Fail loudly**: Clear error messages when violated

### Implementation Strategy

1. **Static Analysis**: Script that greps for unsafe patterns
2. **Unit Tests**: Test the resolver module thoroughly
3. **Integration Tests**: Test end-to-end provider resolution

## DECIDE

### Phase 1: Create Static Analysis Script

Create a shell script that fails if unsafe patterns are found in production code.

### Phase 2: Add Unit Tests for Resolver

Add comprehensive tests for `WorkspaceProviderResolver`.

### Phase 3: Add Integration Tests

Test the full chat and query flows.

## ACT

Creating static analysis script and resolver tests.
