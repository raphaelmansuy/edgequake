/**
 * Runtime browser helpers.
 *
 * WHY: Centralize automated-browser detection so E2E stability rules stay DRY
 * and do not leak ad-hoc heuristics across hooks and providers.
 */

export function isAutomatedBrowser(): boolean {
  if (typeof window === 'undefined' || typeof navigator === 'undefined') {
    return false;
  }

  const globalWindow = window as Window & typeof globalThis & {
    __PLAYWRIGHT__?: boolean;
    Cypress?: unknown;
  };

  return Boolean(
    navigator.webdriver ||
      globalWindow.__PLAYWRIGHT__ ||
      globalWindow.Cypress ||
      /Playwright|HeadlessChrome/i.test(navigator.userAgent)
  );
}

export function shouldAutoConnectRealtime(): boolean {
  return !isAutomatedBrowser();
}

export function getAutomationAwareRefetchInterval(
  standardInterval: number | false,
  automatedInterval: number | false = false
): number | false {
  return isAutomatedBrowser() ? automatedInterval : standardInterval;
}
