# OODA 17 — Observe/Orient/Decide/Act: TypeScript SDK Fix

## Observe
- TypeScript chat types already correct (`message: string`)
- 14 conversation/folder tests were SKIPPED due to empty `E2E_TENANT_ID` and `E2E_USER_ID` defaults
- Chat tests failed with "Invalid workspace ID" because `E2E_WORKSPACE` defaulted to "default" (non-UUID)

## Orient
- Root cause of skips: helper defaults `E2E_TENANT_ID`/`E2E_USER_ID` to `""` → `hasTenantUser` is false
- Root cause of chat failures: `workspaceId: "default"` sent as `X-Workspace-ID: default` → API rejects non-UUID

## Decide
1. Default `E2E_TENANT_ID` to `00000000-0000-0000-0000-000000000002`
2. Default `E2E_USER_ID` to `00000000-0000-0000-0000-000000000001`
3. Default `E2E_WORKSPACE` to empty string (not "default")

## Act
- Fixed `sdks/typescript/tests/e2e/helpers.ts`:
  - `E2E_TENANT_ID` → defaults to migration-created tenant UUID
  - `E2E_USER_ID` → defaults to migration-created user UUID
  - `E2E_WORKSPACE` → defaults to empty string

## Results
**62/62 passed, 0 failed, 0 skipped** (13.04s)
