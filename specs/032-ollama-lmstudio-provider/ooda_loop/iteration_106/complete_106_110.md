# OODA Iterations 106-110: Multi-Tenant Testing

## Iteration 106: Different Workspaces Same Tenant

**Setup**:

- Workspace A: OpenAI
- Workspace B: Ollama

**Test**: Query each workspace sequentially
**Expected**: Correct provider per workspace
**Result**: ✅ Each uses its configured provider

## Iteration 107: Workspace Switch Mid-Session

**Test**: Start with Workspace A, switch to B mid-conversation
**Expected**: New workspace's provider used for new queries
**Result**: ✅ Provider switches correctly

## Iteration 108: Cross-Tenant Workspace Access

**Test**: Query workspace from different tenant
**Expected**: 403 Forbidden
**Result**: ✅ Access denied correctly

## Iteration 109: Stale Workspace ID

**Test**: Query with deleted workspace ID
**Expected**: Workspace not found, fallback to default
**Result**: ✅ Warning logged, server default used

## Iteration 110: Workspace Provider Validation on Create

**Test**: Create workspace with invalid provider name
**Expected**: Validation error during save
**Result**: ✅ Provider must be valid (ollama/openai/lmstudio)
