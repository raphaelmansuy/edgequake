/**
 * SPEC-020 auth UI probe — conditional when auth is enabled in build.
 */
import { expect, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./app-ready";

const DEFAULT_USERNAME = process.env.E2E_USERNAME ?? "admin";
const DEFAULT_PASSWORD = process.env.E2E_PASSWORD ?? "EdgeQuake1";

export async function probeLoginPage(
  page: Page,
): Promise<{ authEnabled: boolean; hasSignIn: boolean }> {
  await page.goto("/login", GOTO_OPTS);
  const username = page.locator("input#username, [role='textbox'][name*='username' i]");
  const hasSignIn = await username.isVisible({ timeout: 5_000 }).catch(() => false);
  return { authEnabled: hasSignIn, hasSignIn };
}

export async function assertLoginFormRenders(page: Page): Promise<void> {
  const { hasSignIn } = await probeLoginPage(page);
  if (!hasSignIn) return;
  await expect(page.locator("input#password, [role='textbox'][name*='password' i]").first()).toBeVisible();
  await expect(page.getByRole("button", { name: /sign in/i }).first()).toBeVisible();
}

/** Dev login used when SPEC020_AUTH_PROOF=1 and auth is enabled. */
export async function performDevLogin(
  page: Page,
  credentials?: { username: string; password: string },
): Promise<{ loggedIn: boolean; landedUrl: string }> {
  const user = credentials?.username ?? DEFAULT_USERNAME;
  const pass = credentials?.password ?? DEFAULT_PASSWORD;

  await page.goto("/login", GOTO_OPTS);
  const username = page.locator("input#username").first();
  const password = page.locator("input#password").first();
  const submit = page.locator("button[type='submit'], button:has-text('Sign in')").first();

  if (!(await username.isVisible({ timeout: 8_000 }).catch(() => false))) {
    return { loggedIn: false, landedUrl: page.url() };
  }

  await username.fill(user);
  await password.fill(pass);
  await submit.click();

  await expect
    .poll(async () => page.url(), {
      timeout: 20_000,
      message: "login should navigate away from /login",
    })
    .not.toContain("/login");

  await expect(page.locator("main").first()).toBeVisible({ timeout: 15_000 });
  return { loggedIn: true, landedUrl: page.url() };
}

export function authProofRequired(): boolean {
  return process.env.SPEC020_AUTH_PROOF === "1";
}
