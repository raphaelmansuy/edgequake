# Lens — Front

Parent: [README](../README.md) · Spec: [13-frontend-architecture](../13-frontend-architecture.md)

## View

The UI does not invent policy. It reads `rag_profile_capabilities` and the constants module. Payload builder adds profile fields on create and omits them on update. `updateWorkspace` strips them if a caller spreads a workspace object.

Wizard steps are data in `stepsForWizard`, not conditionals inside each step. Normal skips extract budget and extraction by not listing those steps.

## Risks

| Risk | What we do |
|------|------------|
| A component compares `rag_profile === 'normal'` and drifts from the server | Review rule: only the label card may read the enum. Behavior reads booleans. |
| Persisted query mode `mix` sticks after switching workspace | Composer resets hidden modes. Kind chips are keyed by workspace id. |
| OpenAPI types and hand-written `Workspace` diverge | Regen `schema.d.ts`. Hand-written request types gain the fields in the same PR. |

## Acceptance

- `buildWorkspaceIngestPayload` tests for create vs update.
- `modesForProfile` unit test.
- Locale parity test.
- Playwright covers wizard, query chips, knowledge route, and workspace switch.

## Laws owned

LAW-180-11. F-180-22.
