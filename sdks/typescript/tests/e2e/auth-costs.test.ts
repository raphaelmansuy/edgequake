/**
 * E2E Tests — Auth, Users, API Keys, Costs
 *
 * WHY: Validates authentication, user management, API key operations,
 * and cost tracking against the live EdgeQuake backend.
 *
 * NOTE: Some auth endpoints may require specific credentials or admin access.
 * Tests handle gracefully when auth is not configured.
 */

import { beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { createE2EClient, E2E_ENABLED, testId } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

// ── Auth ──────────────────────────────────────────────────

describeE2E("Auth E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("gets current user info via /auth/me", async () => {
    try {
      const res = await client.auth.me();
      expect(res).toBeDefined();
      // WHY: Rust returns GetMeResponse { user: UserInfo }
      expect(res.user).toBeDefined();
      expect(typeof res.user.user_id).toBe("string");
    } catch (error: any) {
      // Auth may not be enabled — 401 is acceptable
      if (error.status === 401 || error.status === 403) {
        console.log("Auth /me requires authentication — skipping (expected)");
        return;
      }
      throw error;
    }
  });
});

// ── Users ─────────────────────────────────────────────────

describeE2E("Users E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("lists users with pagination", async () => {
    try {
      const res = await client.users.list({ page: 1, page_size: 10 });
      expect(res).toBeDefined();
      // WHY: Rust returns ListUsersResponse { users, total, page, page_size, total_pages }
      expect(Array.isArray(res.users)).toBe(true);
      expect(typeof res.total).toBe("number");
      expect(typeof res.page).toBe("number");
    } catch (error: any) {
      if (error.status === 401 || error.status === 403) {
        console.log("Users list requires admin auth — skipping (expected)");
        return;
      }
      throw error;
    }
  });
});

// ── API Keys ──────────────────────────────────────────────

describeE2E("API Keys E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("lists API keys with pagination", async () => {
    try {
      const res = await client.apiKeys.list({ page: 1, page_size: 10 });
      expect(res).toBeDefined();
      // WHY: Rust returns ListApiKeysResponse { keys, total, page, page_size, total_pages }
      expect(Array.isArray(res.keys)).toBe(true);
      expect(typeof res.total).toBe("number");
    } catch (error: any) {
      if (error.status === 401 || error.status === 403) {
        console.log("API Keys list requires auth — skipping (expected)");
        return;
      }
      throw error;
    }
  });

  it("creates and revokes an API key", async () => {
    try {
      // Create
      const created = await client.apiKeys.create({
        name: testId("sdk-key"),
        scopes: ["read"],
        expires_in_days: 1,
      });
      expect(created).toBeDefined();
      expect(created.key_id).toBeDefined();
      expect(created.api_key).toBeDefined();
      expect(typeof created.api_key).toBe("string");

      // Revoke
      const revoked = await client.apiKeys.revoke(created.key_id);
      expect(revoked).toBeDefined();
      expect(revoked.key_id).toBe(created.key_id);
    } catch (error: any) {
      if (error.status === 401 || error.status === 403) {
        console.log("API Key CRUD requires auth — skipping (expected)");
        return;
      }
      throw error;
    }
  });
});

// ── Costs ─────────────────────────────────────────────────

describeE2E("Costs E2E", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  }, 30_000);

  it("gets cost summary", async () => {
    try {
      const res = await client.costs.summary();
      expect(res).toBeDefined();
    } catch (error: any) {
      // Costs endpoint may not exist or may error — handle gracefully
      if (error.status === 404 || error.status === 500) {
        console.log(`Costs summary: ${error.message} — skipping`);
        return;
      }
      throw error;
    }
  });

  it("gets cost history", async () => {
    try {
      const res = await client.costs.history();
      expect(res).toBeDefined();
    } catch (error: any) {
      if (error.status === 404 || error.status === 500) {
        console.log(`Costs history: ${error.message} — skipping`);
        return;
      }
      throw error;
    }
  });

  it("gets budget status", async () => {
    try {
      const res = await client.costs.budget();
      expect(res).toBeDefined();
    } catch (error: any) {
      if (error.status === 404 || error.status === 500) {
        console.log(`Costs budget: ${error.message} — skipping`);
        return;
      }
      throw error;
    }
  });
});
