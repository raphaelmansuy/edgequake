# OODA 61: Orient

## Technical Analysis

### Option 1: Remove TenantGuard from Deeplink Pages

**Approach**: Don't wrap deeplink pages in TenantGuard

**Pros**:

- Simple fix
- Deeplink pages handle their own workspace loading

**Cons**:

- Duplicates tenant auto-selection logic
- May have inconsistent UX across pages
- Loses TenantGuard benefits (create dialogs, error states)

### Option 2: Pass Workspace to TenantGuard as Prop

**Approach**: `<TenantGuard workspaceId={workspace?.id}>`

**Pros**:

- TenantGuard knows workspace is already loaded
- Can skip workspace check when prop is provided

**Cons**:

- Requires workspace to be loaded before TenantGuard renders
- Creates chicken-and-egg problem

### Option 3: TenantGuard Reads URL for Context

**Approach**: TenantGuard checks if current URL is `/w/[slug]/*`

**Pros**:

- No prop passing needed
- TenantGuard can wait for deeplink resolution

**Cons**:

- Couples TenantGuard to route structure
- May become complex with more route patterns

### Option 4: Deeplink Pages Bypass TenantGuard (✅ CHOSEN)

**Approach**: Deeplink pages don't use TenantGuard, they handle everything themselves

**Pros**:

- Clean separation of concerns
- Deeplink pages are self-contained
- No race conditions with TenantGuard

**Cons**:

- Need to duplicate some loading/error UI
- Slight code duplication

### Implementation Plan

1. Remove `TenantGuard` wrapper from deeplink pages
2. Add proper loading/error states directly in deeplink pages
3. Keep tenant auto-selection logic (already implemented)
4. Test E2E to verify fix
