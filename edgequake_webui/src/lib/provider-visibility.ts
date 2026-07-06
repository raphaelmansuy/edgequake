/**
 * UI provider visibility (SPEC-043) — mirrors backend provider_visibility.rs.
 */
export const UI_HIDDEN_PROVIDER_IDS = new Set(["mock", "mock-imagegen"]);

export function isMockProvider(providerId: string): boolean {
  return UI_HIDDEN_PROVIDER_IDS.has(providerId.trim().toLowerCase());
}

export function isUiVisibleProviderId(providerId: string): boolean {
  return !isMockProvider(providerId);
}
