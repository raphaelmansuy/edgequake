import { describe, expect, it } from "vitest";
import { toAccess } from "../use-access";
import type { MeAccessResponse } from "@/lib/api/edgequake";

/**
 * `toAccess` maps `GET /me/access` into the `Access` the whole shell renders from, and it is
 * called on every render of a dashboard route. It used to dereference `payload.platform`
 * directly, so a 200 that carried anything else threw during render and replaced the page with
 * the error boundary:
 *
 *     Cannot read properties of undefined (reading 'role')
 *
 * The e2e hermetic guard returns `{ error: { code, message } }` for any `/api/v1/**` route it
 * has not mocked, which is exactly that shape. One unregistered access probe therefore blanked
 * `/documents` and took every chromium spec that opens the page down with it.
 *
 * An unreadable access payload must degrade to "no access" — a state the shell already renders.
 */
describe("toAccess", () => {
  it("returns NO_ACCESS instead of throwing when platform is missing", () => {
    const payload = { error: { code: "spec155_unmocked", message: "No mock registered" } };
    expect(() => toAccess(payload as unknown as MeAccessResponse)).not.toThrow();

    const access = toAccess(payload as unknown as MeAccessResponse);
    expect(access.platform.admin).toBe(false);
    expect(access.organization).toBeNull();
    expect(access.workspaces).toEqual([]);
    expect(access.canCreate.organization).toBe(false);
    expect(access.canCreate.workspace).toBe(false);
  });

  it("returns NO_ACCESS for a null body", () => {
    expect(() => toAccess(null as unknown as MeAccessResponse)).not.toThrow();
    expect(toAccess(null as unknown as MeAccessResponse).platform.admin).toBe(false);
  });

  it("still maps a real payload", () => {
    const access = toAccess({
      platform: { role: "admin", admin: true, capabilities: ["platform.connections"] },
      organization: {
        id: "t1",
        name: "Acme",
        slug: "acme",
        role: "owner",
        level: 4,
        is_active: true,
        is_archived: false,
        capabilities: ["organization.members"],
      },
      workspaces: [
        { id: "w1", name: "Legal", slug: "legal", role: "admin", level: 3, inherited: false, capabilities: [] },
      ],
      quota: { workspaces: { used: 1, limit: 10 }, seats: { used: 2, limit: 25 } },
      can_create: { organization: true, workspace: true },
      can_delete: { organization: true },
      reason: { organization: null, workspace: null },
    } as unknown as MeAccessResponse);

    expect(access.platform.admin).toBe(true);
    expect(access.organization?.role).toBe("owner");
    expect(access.workspaces[0]?.id).toBe("w1");
  });
});
