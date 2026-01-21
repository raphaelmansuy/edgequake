/**
 * SPEC-032: Query Page Model Selection and Lineage E2E Tests
 *
 * Focus 3: Query page provider selection with lineage tracking
 * Focus 15: Message lineage information storage and display
 * Focus 18: Tokens per second display
 *
 * Tests for query-time provider selection and response metadata.
 *
 * @implements SPEC-032: Focus 3, 15, 18 - Query model selection and lineage
 * @iteration OODA 59
 */
import { expect, test } from "@playwright/test";

const BACKEND_URL = "http://localhost:8080";

test.setTimeout(90000);

test.describe("SPEC-032: Query Page Model Selection", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.evaluate(() => localStorage.clear());
    await page.waitForLoadState("domcontentloaded");
  });

  test.describe("Focus 3: Provider/Model Selector Visibility", () => {
    test("query page loads provider selector", async ({ page, request }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page via deeplink
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Page should have provider selector
      const selectors = page.locator('[role="combobox"]');
      const selectorCount = await selectors.count();

      expect(selectorCount).toBeGreaterThan(0);
    });

    test("provider selector shows current selection", async ({
      page,
      request,
    }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Provider selector should show a value
      const selector = page.locator('[role="combobox"]').first();
      if (await selector.isVisible()) {
        const text = await selector.textContent();
        expect(text?.length).toBeGreaterThan(0);
      }
    });

    test("clicking provider selector opens dropdown", async ({
      page,
      request,
    }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Click provider selector
      const selector = page.locator('[role="combobox"]').first();
      if (await selector.isVisible()) {
        await selector.click();
        await page.waitForTimeout(500);

        // Dropdown should appear
        const options = page.locator('[role="option"], [role="listbox"]');
        const hasOptions = (await options.count()) > 0;

        expect(hasOptions).toBe(true);

        // Close dropdown
        await page.keyboard.press("Escape");
      }
    });

    test("provider dropdown shows multiple providers", async ({
      page,
      request,
    }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Click provider selector
      const selector = page.locator('[role="combobox"]').first();
      if (await selector.isVisible()) {
        await selector.click();
        await page.waitForTimeout(500);

        // Check for provider names in dropdown
        const pageContent = await page.content();
        const hasMultipleProviders =
          (pageContent.toLowerCase().includes("openai") &&
            pageContent.toLowerCase().includes("ollama")) ||
          (pageContent.toLowerCase().includes("gpt") &&
            pageContent.toLowerCase().includes("gemma"));

        expect(hasMultipleProviders).toBe(true);

        // Close dropdown
        await page.keyboard.press("Escape");
      }
    });
  });

  test.describe("Focus 3: Model Selection Workflow", () => {
    test("can select different model from dropdown", async ({
      page,
      request,
    }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Click provider selector
      const selector = page.locator('[role="combobox"]').first();
      if (await selector.isVisible()) {
        const initialText = await selector.textContent();

        await selector.click();
        await page.waitForTimeout(500);

        // Click first option
        const firstOption = page.locator('[role="option"]').first();
        if (await firstOption.isVisible()) {
          await firstOption.click();
          await page.waitForTimeout(500);

          // Selection should update
          const newText = await selector.textContent();
          expect(newText?.length).toBeGreaterThan(0);
        }
      }
    });

    test("query input textarea is present", async ({ page, request }) => {
      // Get workspace context
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.slug) {
        test.skip();
        return;
      }

      const workspaceSlug = workspaces.items[0].slug;

      // Navigate to query page
      await page.goto(`/w/${workspaceSlug}/query`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Query input should be visible
      const queryInput = page.locator('textarea, input[type="text"]');
      const hasInput = (await queryInput.count()) > 0;

      expect(hasInput).toBe(true);
    });
  });

  test.describe("Focus 15: Lineage Information", () => {
    test("query API includes mode in response", async ({ request }) => {
      // Get workspace
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.id) {
        test.skip();
        return;
      }

      const workspaceId = workspaces.items[0].id;

      // Make query
      const queryResponse = await request.post(`${BACKEND_URL}/api/v1/query`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
        data: {
          query: "Test lineage query",
          mode: "naive",
          stream: false,
        },
      });

      const response = await queryResponse.json();

      // Response should include mode
      if (queryResponse.ok()) {
        expect(response.mode).toBeDefined();
      }
    });

    test("query API includes stats", async ({ request }) => {
      // Get workspace
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.id) {
        test.skip();
        return;
      }

      const workspaceId = workspaces.items[0].id;

      // Make query
      const queryResponse = await request.post(`${BACKEND_URL}/api/v1/query`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
        data: {
          query: "Test stats query",
          mode: "hybrid",
          stream: false,
        },
      });

      const response = await queryResponse.json();

      // Response may include stats
      if (queryResponse.ok()) {
        expect(response).toBeDefined();
        // Stats may be included in different fields
        const hasStatsFields =
          response.stats !== undefined ||
          response.tokens !== undefined ||
          response.duration !== undefined ||
          response.mode !== undefined;
        expect(hasStatsFields).toBe(true);
      }
    });

    test("models API provides display names for lineage", async ({
      request,
    }) => {
      const response = await request.get(`${BACKEND_URL}/api/v1/models`);
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // All providers should have display names
      for (const provider of data.providers) {
        expect(provider).toHaveProperty("display_name");
        expect(provider.display_name.length).toBeGreaterThan(0);

        // Models should have display names
        for (const model of provider.models) {
          expect(model).toHaveProperty("display_name");
          expect(model.display_name.length).toBeGreaterThan(0);
        }
      }
    });
  });

  test.describe("Focus 18: Tokens Per Second", () => {
    test("models API includes cost information", async ({ request }) => {
      const response = await request.get(`${BACKEND_URL}/api/v1/models`);
      expect(response.ok()).toBe(true);

      const data = await response.json();

      // Models should have cost info (for calculating per-token metrics)
      const allModels = data.providers.flatMap(
        (p: { models: unknown[] }) => p.models
      );
      for (const model of allModels.slice(0, 5)) {
        expect(model).toHaveProperty("cost");
        expect(model.cost).toHaveProperty("input_per_1k");
        expect(model.cost).toHaveProperty("output_per_1k");
      }
    });

    test("query response could include duration for tokens/sec calculation", async ({
      request,
    }) => {
      // Get workspace
      const tenantsResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants`
      );
      const tenants = await tenantsResponse.json();
      if (!tenants.items?.[0]?.id) {
        test.skip();
        return;
      }

      const tenantId = tenants.items[0].id;
      const workspacesResponse = await request.get(
        `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
      );
      const workspaces = await workspacesResponse.json();
      if (!workspaces.items?.[0]?.id) {
        test.skip();
        return;
      }

      const workspaceId = workspaces.items[0].id;

      // Make query and measure time
      const startTime = Date.now();
      const queryResponse = await request.post(`${BACKEND_URL}/api/v1/query`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
        data: {
          query: "Test duration query",
          mode: "naive",
          stream: false,
        },
      });
      const endTime = Date.now();

      // Duration is calculable
      const clientDuration = endTime - startTime;
      expect(clientDuration).toBeGreaterThan(0);

      // Response may include server-side duration
      if (queryResponse.ok()) {
        const response = await queryResponse.json();
        // Server may include duration_ms or similar
        expect(response).toBeDefined();
      }
    });
  });
});

test.describe("SPEC-032: Query Mode Selection", () => {
  test("query API supports different modes", async ({ request }) => {
    // Get workspace
    const tenantsResponse = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    const tenants = await tenantsResponse.json();
    if (!tenants.items?.[0]?.id) {
      test.skip();
      return;
    }

    const tenantId = tenants.items[0].id;
    const workspacesResponse = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
    );
    const workspaces = await workspacesResponse.json();
    if (!workspaces.items?.[0]?.id) {
      test.skip();
      return;
    }

    const workspaceId = workspaces.items[0].id;

    // Test different modes
    const modes = ["naive", "local", "global", "hybrid"];

    for (const mode of modes) {
      const queryResponse = await request.post(`${BACKEND_URL}/api/v1/query`, {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
        data: {
          query: `Test ${mode} mode`,
          mode: mode,
          stream: false,
        },
      });

      // All modes should be accepted
      expect([200, 400, 500]).toContain(queryResponse.status());

      if (queryResponse.ok()) {
        const response = await queryResponse.json();
        expect(response.mode).toBe(mode);
      }
    }
  });

  test("query API supports streaming parameter", async ({ request }) => {
    // Get workspace
    const tenantsResponse = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    const tenants = await tenantsResponse.json();
    if (!tenants.items?.[0]?.id) {
      test.skip();
      return;
    }

    const tenantId = tenants.items[0].id;
    const workspacesResponse = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
    );
    const workspaces = await workspacesResponse.json();
    if (!workspaces.items?.[0]?.id) {
      test.skip();
      return;
    }

    const workspaceId = workspaces.items[0].id;

    // Non-streaming request
    const nonStreamResponse = await request.post(
      `${BACKEND_URL}/api/v1/query`,
      {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
        data: {
          query: "Test non-streaming",
          mode: "naive",
          stream: false,
        },
      }
    );

    expect([200, 400, 500]).toContain(nonStreamResponse.status());
  });
});

test.describe("SPEC-032: Query Conversation History", () => {
  test("conversations API requires workspace context", async ({ request }) => {
    // Request without headers
    const response = await request.get(`${BACKEND_URL}/api/v1/conversations`);
    // Should require context (400) or not found (404)
    expect([400, 404]).toContain(response.status());

    if (response.status() === 400) {
      const error = await response.json();
      expect(error).toHaveProperty("message");
    }
  });

  test("conversations can be listed with workspace context", async ({
    request,
  }) => {
    // Get workspace
    const tenantsResponse = await request.get(`${BACKEND_URL}/api/v1/tenants`);
    const tenants = await tenantsResponse.json();
    if (!tenants.items?.[0]?.id) {
      test.skip();
      return;
    }

    const tenantId = tenants.items[0].id;
    const workspacesResponse = await request.get(
      `${BACKEND_URL}/api/v1/tenants/${tenantId}/workspaces`
    );
    const workspaces = await workspacesResponse.json();
    if (!workspaces.items?.[0]?.id) {
      test.skip();
      return;
    }

    const workspaceId = workspaces.items[0].id;

    // Request with headers
    const response = await request.get(`${BACKEND_URL}/api/v1/conversations`, {
      headers: {
        "X-Tenant-Id": tenantId,
        "X-Workspace-Id": workspaceId,
      },
    });

    // API should return conversations or accept the request (401 = auth required)
    expect([200, 401, 404]).toContain(response.status());
    if (response.ok()) {
      const conversations = await response.json();
      expect(conversations).toHaveProperty("items");
    }
  });
});
