/**
 * SPEC-158 — map backend SSO denial/conflict codes (`?error=<code>`) to i18n keys.
 * Codes are stable API contract (specs/158-.../08-api-and-token-contract.md); unknown codes fall
 * back to a generic message so a new server code can never render raw.
 */

export const SSO_ERROR_KEYS: Record<string, string> = {
  invalid_handoff: "auth.sso.errors.invalidHandoff",
  org_unknown: "auth.sso.errors.orgUnknown",
  org_missing: "auth.sso.errors.orgMissing",
  org_ambiguous: "auth.sso.errors.orgAmbiguous",
  tenant_suspended: "auth.sso.errors.tenantSuspended",
  hd_mismatch: "auth.sso.errors.hdMismatch",
  idp_tenant_not_allowed: "auth.sso.errors.idpTenantNotAllowed",
  jit_disabled: "auth.sso.errors.jitDisabled",
  max_users: "auth.sso.errors.maxUsers",
  membership_revoked: "auth.sso.errors.membershipRevoked",
  tenant_access_denied: "auth.sso.errors.tenantAccessDenied",
  account_exists_unlinked: "auth.sso.errors.accountExistsUnlinked",
  access_denied: "auth.sso.errors.accessDenied",
  sso_unavailable: "auth.sso.errors.ssoUnavailable",
};

export const SSO_ERROR_FALLBACK_KEY = "auth.sso.errors.generic";

/** Strip `:detail` (e.g. `org_ambiguous:acme,globex`) — detail is only for the picker, not for display. */
export function ssoErrorBase(code: string): string {
  return code.split(":", 1)[0] ?? code;
}

export function ssoErrorKey(code: string | null | undefined): string {
  if (!code) return SSO_ERROR_FALLBACK_KEY;
  return SSO_ERROR_KEYS[ssoErrorBase(code)] ?? SSO_ERROR_FALLBACK_KEY;
}

/** Org aliases offered by an `org_ambiguous:a,b` denial (empty otherwise). */
export function ambiguousOrgAliases(code: string | null | undefined): string[] {
  if (!code?.startsWith("org_ambiguous:")) return [];
  return code
    .slice("org_ambiguous:".length)
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}
