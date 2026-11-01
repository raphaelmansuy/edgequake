# SPEC-181 — e2e harness

Hermetic Playwright, no backend, no LLM. Mirrors the
[SPEC-169](../../169-perfect-user-management/e2e) pattern so the two suites share helpers and
review habits.

## Layout

```text
edgequake_webui/e2e/spec181/
  helpers/
    admin-mock.ts        stateful mock of tenancy + access + grants + purge
    admin-data.ts        deterministic fixtures (4 personas, 2 orgs, 5 workspaces)
    shots.ts             re-export of spec169/helpers/shots (which re-exports spec168)
  create-organization.spec.ts
  create-workspace.spec.ts
  access-gating.spec.ts
  grants.spec.ts
  navigation.spec.ts
  destructive.spec.ts
  a11y.spec.ts
```

Screenshots land in `specs/181-admin/e2e/screenshots/<SPEC181_ROUND>/`.

## Run

```bash
cd edgequake_webui

# whole pack, writes round-01 screenshots
SPEC181_ROUND=round-01 npx playwright test e2e/spec181 --project=mock-api

# one flow
npx playwright test e2e/spec181/create-workspace.spec.ts --project=mock-api

# accessibility only
npx playwright test e2e/spec181/a11y.spec.ts --project=mock-api
```

The `mock-api` project exists already and skips the live-stack check
(`PLAYWRIGHT_SKIP_STACK_CHECK=1`, Next dev on :3010).

## Mock contract

`admin-mock.ts` intercepts:

| Route | Behaviour |
|-------|-----------|
| `GET /api/v1/me/access` | derived from `opts.viewer`; sets `can_create`, quota, per-workspace capabilities |
| `GET /api/v1/tenants` | orgs visible to the viewer |
| `POST /api/v1/tenants` | **creates** (mutates state), 201, writes an owner grant |
| `GET /api/v1/tenants/{t}/workspaces` | live + archived split |
| `POST /api/v1/tenants/{t}/workspaces` | **creates**, honours quota → 201 or 409 |
| `GET/POST/PATCH/DELETE /api/v1/tenants/{t}/grants` | **mutates**, enforces rank + cross-axis |
| `POST /api/v1/tenants/{t}/invitations` | returns a token; records `workspace_id` |
| `DELETE /api/v1/tenants/{t}` | 202 + receipt; then planes finish on subsequent polls |
| `POST /api/v1/workspaces/{w}/archive` | sets archived, frees quota |
| `GET /api/v1/roles` | the 15 capabilities with `axis` |

Options:

```ts
type Viewer = 'platform' | 'owner' | 'orgadmin' | 'wsadmin' | 'member' | 'viewer';

interface AdminMockOptions {
  viewer?: Viewer;              // default 'owner'
  quota?: { used: number; limit: number };
  seats?: { used: number; limit: number };
  orgCount?: number;            // default 1; 0 exercises W-02
  workspaceCount?: number;      // default 3
  forceError?: 'create' | 'grant' | 'purge';
  purgePlanesFail?: string[];   // e.g. ['originals']
}
```

The mock is **stateful** on purpose. `E-success-clears-draft` and `E-two-grant-rows` are only
meaningful if a create and a grant actually change the next response.

## Fixtures

```text
  Acme Research (org)          Globex Holdings (org)
    Legal     graph             Support     normal
    Finance   normal            Ops         graph
    Research  graph
    Legacy    archived

  people
    ada   platform admin, owner of both orgs
    marc  owner of Acme
    ines  org admin of Acme
    aicha member of Acme + admin of Legal   ← the two-grant case
    tom   member of Legal only              ← no org grant
    ruth  viewer of Legal
```

These six cover every capability cell in [05 §3](../05-authorization-model.md#3--the-capability-catalog).

## Assertion conventions

| Rule | Why |
|------|-----|
| Assert absence with `toHaveCount(0)` | a visually hidden element still exists; this pack's promise is that it does not |
| Never assert a toast as the only proof of a create | assert the row, the quota, and the URL |
| Every spec that mutates state starts from `prepareAdminPage(page, opts)` | deterministic baseline |
| Screenshots use `animations: 'disabled'` | comparable across rounds |
| One `test.describe` per flow, `test.setTimeout(120_000)` | dialog + screenshot overhead |
| Tag with `@spec181` | the default `chromium` project excludes it |

## Live smoke (manual, before merge)

```bash
make dev-bg
curl -s localhost:8080/health | jq .status          # healthy
# browser: create an org as platform admin, confirm Owner row in members
# browser: create a workspace as org owner in 3 decisions
# browser: archive it, confirm quota drops
# browser: delete the org, watch 9 planes
python3 specs/181-admin/scripts/validate-cross-ref.py specs/181-admin
```

The hermetic suite proves the screens; the live smoke proves the transaction. Neither replaces
the other.
