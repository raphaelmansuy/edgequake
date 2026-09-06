/**
 * Append SPEC-146 security fields onto upload FormData (DRY SSOT).
 */

import {
  securityFieldsToRecord,
  type SecurityFields,
} from "@/lib/security/security-fields";

/** Multipart field names recognized by backend SecurityFormOverrides. */
export function appendSecurityFields(
  formData: FormData,
  fields?: SecurityFields | null,
): void {
  if (!fields) return;
  const record = securityFieldsToRecord(fields);
  for (const [key, value] of Object.entries(record)) {
    formData.append(key, String(value));
  }
}

/** Merge security fields into a metadata object (text/JSON upload path). */
export function mergeSecurityIntoMetadata(
  metadata: Record<string, unknown> | undefined,
  fields?: SecurityFields | null,
): Record<string, unknown> | undefined {
  if (!fields) return metadata;
  return {
    ...(metadata ?? {}),
    ...securityFieldsToRecord(fields),
  };
}
