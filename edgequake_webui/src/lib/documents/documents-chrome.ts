/**
 * Documents page chrome height SSOT (SPEC-099 inventory scroll + SPEC-146 Security).
 *
 * Collapsed Security (or ABAC off) keeps a tight chrome budget so inventory stays usable.
 * Expanded SecurityFields needs more room so Classification/Share are not clipped.
 */
export const DOCUMENTS_CHROME_MAX_COLLAPSED = 'max-h-[42dvh]' as const;
export const DOCUMENTS_CHROME_MAX_SECURITY_OPEN = 'max-h-[72dvh]' as const;

export function documentsChromeMaxClass(securityExpanded: boolean): string {
  return securityExpanded
    ? DOCUMENTS_CHROME_MAX_SECURITY_OPEN
    : DOCUMENTS_CHROME_MAX_COLLAPSED;
}
