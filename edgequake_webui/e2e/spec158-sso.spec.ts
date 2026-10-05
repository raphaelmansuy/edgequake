/**
 * @spec158 — Enterprise SSO through a real Keycloak (Organizations = tenants).
 *
 * Opt-in: needs `make dev-sso` (or the equivalent) and E2E_SSO=1. Demo users come from
 * deploy/keycloak/realm/edgequake-realm.json; password = E2E_SSO_PASSWORD (EQ_KC_DEMO_PASSWORD).
 * Tenants `acme` and `globex` must exist (`scripts/keycloak_smoke.py --api ... --admin-password ...` seeds them).
 */
import { expect, test, type Page } from "@playwright/test";

const enabled = process.env.E2E_SSO === "1";
const password = process.env.E2E_SSO_PASSWORD ?? "demo-password-change-me";

/** Keycloak 26 login is two-step (username, then password) — answer whichever step is shown. */
async function keycloakLogin(page: Page, username: string) {
  await page.waitForURL(/\/realms\/edgequake\//, { timeout: 30_000 });
  await page.locator("#username").fill(username);
  const pwd = page.locator("#password");
  if (!(await pwd.isVisible().catch(() => false))) {
    await page.locator("#kc-login").click();
  }
  await pwd.fill(password);
  await page.locator("#kc-login").click();
}

function isAppUrl(url: URL, base: string): boolean {
  const expected = new URL(base);
  const loopback = new Set(["localhost", "127.0.0.1"]);
  const sameHost =
    url.hostname === expected.hostname ||
    (loopback.has(url.hostname) && loopback.has(expected.hostname));
  return sameHost && url.port === expected.port && !/^\/(login|auth\/callback)/.test(url.pathname);
}

async function expectSignedIn(page: Page) {
  const base = test.info().project.use.baseURL ?? "http://localhost:3000";
  await page.waitForURL((u) => isAppUrl(u, base), {
    timeout: 30_000,
  });
  expect(page.url()).not.toMatch(/[?&](code|error|access_token|id_token)=/);
  const stored = await page.evaluate(() => ({
    access: localStorage.getItem("accessToken"),
    refresh: localStorage.getItem("refreshToken"),
  }));
  expect(stored.access).toBeNull();
  expect(stored.refresh).toBeNull();
}

test.describe("@spec158 SSO login", () => {
  test.skip(!enabled, "set E2E_SSO=1 with a running Keycloak + EdgeQuake SSO stack");
  test("login page offers the SSO provider next to the password form", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByTestId("sso-provider-keycloak")).toBeVisible();
    await expect(page.getByLabel(/organization/i)).toBeVisible();
    await expect(page.getByLabel("Username")).toBeVisible();
  });

  test("alice signs in to the acme organization without any token in the URL or localStorage", async ({ page }) => {
    await page.goto("/login");
    await page.getByLabel(/organization/i).fill("acme");
    await page.getByTestId("sso-provider-keycloak").click();
    await keycloakLogin(page, "alice");
    await expectSignedIn(page);
    const tenant = page.getByTestId("context-tenant-label");
    await expect(tenant).toContainText(/acme/i, { timeout: 20_000 });
    await tenant.click();
    await expect(page.getByTestId("tenant-option-acme")).toBeVisible();
    await expect(page.getByTestId("tenant-option-globex")).toHaveCount(0);
  });

  test("carol (two organizations) gets a picker instead of an arbitrary tenant", async ({ page }) => {
    await page.goto("/login");
    await page.getByTestId("sso-provider-keycloak").click();
    await keycloakLogin(page, "carol");
    await expect(page.getByTestId("sso-org-picker")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("sso-error-message")).toContainText(/several organizations/i);
    await page.getByRole("button", { name: "globex" }).click();
    // Keycloak already holds carol's session, so the second round trip is silent.
    await expectSignedIn(page);
  });

});

test.describe("@spec158 SSO callback errors (no IdP)", () => {
  test("a replayed or expired handoff code shows a clear error and a way back", async ({ page }) => {
    await page.route("**/api/v1/auth/handoff", (route) => route.fulfill({
      status: 401,
      contentType: "application/json",
      body: JSON.stringify({ error: { code: "UNAUTHORIZED", message: "code_invalid" } }),
    }));
    await page.goto("/auth/callback?code=not-a-real-code");
    await expect(page.getByTestId("sso-error")).toBeVisible();
    await expect(page.getByTestId("sso-error-message")).toContainText(/expired|already used/i);
    await page.getByRole("button", { name: /back to sign in/i }).click();
    await expect(page).toHaveURL(/\/login/);
  });

  test("unknown denial codes never render raw", async ({ page }) => {
    await page.goto("/auth/callback?error=some_future_code");
    await expect(page.getByTestId("sso-error-message")).toContainText(/single sign-on failed/i);
    await expect(page.getByTestId("sso-error-message")).not.toContainText("some_future_code");
  });

  test("sso_unavailable (EC-158-16) tells the user to use a password", async ({ page }) => {
    await page.goto("/auth/callback?error=sso_unavailable");
    await expect(page.getByTestId("sso-error-message")).toContainText(/unreachable|password/i);
    await expect(page.getByTestId("sso-error-message")).not.toContainText("sso_unavailable");
  });
});
