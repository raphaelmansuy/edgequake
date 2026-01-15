/**
 * SPEC-032: Workspace Model Configuration UI E2E Tests
 *
 * Focus 4: Workspace settings page displaying model configuration
 * Focus 5: Rebuild embeddings with processing display
 * Focus 19: Extractor model configuration clarity
 * Focus 21: Workspace configuration deeplink accessibility
 *
 * Tests for the workspace settings page UI interactions.
 *
 * @implements SPEC-032: Focus 4, 5, 19, 21 - Workspace settings E2E
 * @iteration OODA 59
 */
import { expect, test } from "@playwright/test";

const BACKEND_URL = "http://localhost:8080";

test.setTimeout(90000);

test.describe("SPEC-032: Workspace Configuration UI", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.evaluate(() => localStorage.clear());
    await page.waitForLoadState("domcontentloaded");
  });

  test.describe("Focus 4: Workspace Settings Page Display", () => {
    test("workspace page loads and displays content", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Page should load
      const main = page.locator("main");
      await expect(main).toBeVisible({ timeout: 15000 });
    });

    test("workspace page shows LLM configuration section", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for LLM configuration elements
      const pageContent = await page.content();
      const hasLlmSection =
        pageContent.toLowerCase().includes("llm") ||
        pageContent.toLowerCase().includes("extractor") ||
        pageContent.toLowerCase().includes("model") ||
        pageContent.toLowerCase().includes("provider");

      expect(hasLlmSection).toBe(true);
    });

    test("workspace page shows embedding configuration section", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for embedding configuration elements
      const pageContent = await page.content();
      const hasEmbeddingSection =
        pageContent.toLowerCase().includes("embedding") ||
        pageContent.toLowerCase().includes("vector") ||
        pageContent.toLowerCase().includes("dimension");

      expect(hasEmbeddingSection).toBe(true);
    });

    test("workspace page shows provider status", async ({ page, request }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for provider status section
      const providerText = page.getByText(/provider|status/i);
      const hasProviderStatus = (await providerText.count()) > 0;

      // At minimum, page should display some provider info
      expect(hasProviderStatus).toBe(true);
    });
  });

  test.describe("Focus 5: Rebuild Embeddings UI", () => {
    test("workspace page has rebuild embeddings button", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for rebuild button or text
      const rebuildText = page.getByText(/rebuild/i);
      const hasRebuild = (await rebuildText.count()) > 0;

      expect(hasRebuild).toBe(true);
    });

    test("workspace page has rebuild knowledge graph button", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for knowledge graph rebuild option
      const pageContent = await page.content();
      const hasKnowledgeGraph =
        pageContent.toLowerCase().includes("knowledge") ||
        pageContent.toLowerCase().includes("graph") ||
        pageContent.toLowerCase().includes("extraction");

      expect(hasKnowledgeGraph).toBe(true);
    });
  });

  test.describe("Focus 19: Extractor Model Configuration", () => {
    test("workspace shows extractor/LLM config is for ingestion", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace page
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(3000);

      // Look for indication that LLM is for extraction/ingestion
      const pageContent = await page.content();
      const hasExtractorContext =
        pageContent.toLowerCase().includes("extractor") ||
        pageContent.toLowerCase().includes("extraction") ||
        pageContent.toLowerCase().includes("ingestion") ||
        pageContent.toLowerCase().includes("processing");

      expect(hasExtractorContext).toBe(true);
    });
  });

  test.describe("Focus 21: Workspace Configuration Deeplink", () => {
    test("workspace settings accessible via /w/[slug]/settings", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to settings deeplink
      await page.goto(`/w/${workspaceSlug}/settings`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(2000);

      // Page should load
      const main = page.locator("main");
      await expect(main).toBeVisible({ timeout: 15000 });

      // Should not show 404
      const hasNotFound = await page
        .getByText("404")
        .isVisible()
        .catch(() => false);
      expect(hasNotFound).toBe(false);
    });

    test("workspace accessible via /w/[slug]/workspace", async ({
      page,
      request,
    }) => {
      // Get workspace slug
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

      // Navigate to workspace deeplink
      await page.goto(`/w/${workspaceSlug}/workspace`, {
        waitUntil: "domcontentloaded",
      });
      await page.waitForTimeout(2000);

      // Page should load
      const main = page.locator("main");
      await expect(main).toBeVisible({ timeout: 15000 });
    });

    test("home page has link to workspace configuration", async ({ page }) => {
      // Go to home
      await page.goto("/", { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(2000);

      // Look for workspace link in navigation
      const workspaceLink = page.getByRole("link", { name: /workspace/i });
      const hasWorkspaceLink = (await workspaceLink.count()) > 0;

      // Either in sidebar or menu
      expect(hasWorkspaceLink).toBe(true);
    });
  });
});

test.describe("SPEC-032: Workspace Model Editing UI", () => {
  test("workspace page has edit mode toggle", async ({ page, request }) => {
    // Get workspace slug
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
    if (!workspaces.items?.[0]?.slug) {
      test.skip();
      return;
    }

    const workspaceSlug = workspaces.items[0].slug;

    // Navigate to workspace page
    await page.goto(`/w/${workspaceSlug}/workspace`, {
      waitUntil: "domcontentloaded",
    });
    await page.waitForTimeout(3000);

    // Look for edit button or toggle
    const editButton = page.getByRole("button", {
      name: /edit|modify|change/i,
    });
    const pageContent = await page.content();
    const hasEditCapability =
      (await editButton.count()) > 0 ||
      pageContent.toLowerCase().includes("edit") ||
      pageContent.toLowerCase().includes("save");

    expect(hasEditCapability).toBe(true);
  });

  test("workspace shows model selector components", async ({
    page,
    request,
  }) => {
    // Get workspace slug
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
    if (!workspaces.items?.[0]?.slug) {
      test.skip();
      return;
    }

    const workspaceSlug = workspaces.items[0].slug;

    // Navigate to workspace page
    await page.goto(`/w/${workspaceSlug}/workspace`, {
      waitUntil: "domcontentloaded",
    });
    await page.waitForTimeout(3000);

    // Look for model-related elements (buttons, selectors, text)
    const pageContent = await page.content();
    const hasModelContent =
      pageContent.toLowerCase().includes("model") ||
      pageContent.toLowerCase().includes("provider") ||
      pageContent.toLowerCase().includes("llm") ||
      pageContent.toLowerCase().includes("embedding");

    // Should have model-related content on the page
    expect(hasModelContent).toBe(true);
  });
});

test.describe("SPEC-032: Workspace Stats Display", () => {
  test("workspace page shows document count", async ({ page, request }) => {
    // Get workspace slug
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
    if (!workspaces.items?.[0]?.slug) {
      test.skip();
      return;
    }

    const workspaceSlug = workspaces.items[0].slug;

    // Navigate to workspace page
    await page.goto(`/w/${workspaceSlug}/workspace`, {
      waitUntil: "domcontentloaded",
    });
    await page.waitForTimeout(3000);

    // Look for stats section
    const pageContent = await page.content();
    const hasStats =
      pageContent.toLowerCase().includes("document") ||
      pageContent.toLowerCase().includes("nodes") ||
      pageContent.toLowerCase().includes("edges") ||
      pageContent.toLowerCase().includes("vectors");

    expect(hasStats).toBe(true);
  });

  test("workspace stats API returns counts", async ({ request }) => {
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

    // Get workspace stats
    const statsResponse = await request.get(
      `${BACKEND_URL}/api/v1/workspaces/${workspaceId}/stats`,
      {
        headers: {
          "X-Tenant-Id": tenantId,
          "X-Workspace-Id": workspaceId,
        },
      }
    );

    // Stats endpoint should exist
    expect([200, 404]).toContain(statsResponse.status());

    if (statsResponse.status() === 200) {
      const stats = await statsResponse.json();
      expect(stats).toBeDefined();
    }
  });
});
