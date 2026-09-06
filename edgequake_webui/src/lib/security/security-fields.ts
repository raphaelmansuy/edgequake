/**
 * SPEC-146 SecurityFields — single SSOT for upload labels + table chips.
 * Values mirror edgequake-authz::catalog (CLASSIFICATIONS / SHARE_MODES).
 */

export type Classification =
  | "public"
  | "internal"
  | "confidential"
  | "secret";

export type ShareMode = "workspace" | "acl" | "classified" | "owner_only";
export type SecurityStatus = "ok" | "quarantined";

export interface CatalogOption<T extends string = string> {
  value: T;
  label: string;
}

/** Clearance lattice — keep in sync with edgequake-authz::CLASSIFICATIONS. */
export const CLASSIFICATIONS: CatalogOption<Classification>[] = [
  { value: "public", label: "Public" },
  { value: "internal", label: "Internal" },
  { value: "confidential", label: "Confidential" },
  { value: "secret", label: "Secret" },
];

/** Share modes — keep in sync with edgequake-authz::SHARE_MODES. */
export const SHARE_MODES: CatalogOption<ShareMode>[] = [
  { value: "workspace", label: "Workspace" },
  { value: "acl", label: "ACL" },
  { value: "classified", label: "Classified" },
  { value: "owner_only", label: "Owner only" },
];

export interface SecurityFields {
  classification: Classification | string;
  share_mode: ShareMode;
  export_control: boolean;
  pii: boolean;
  project_id?: string;
  security_status?: SecurityStatus;
  /** When share_mode=acl: principal user ids to grant after admit (UI intent). */
  acl_principal_ids?: string[];
}

export const DEFAULT_SECURITY_FIELDS: SecurityFields = {
  classification: "internal",
  share_mode: "workspace",
  export_control: false,
  pii: false,
  security_status: "ok",
  acl_principal_ids: [],
};

export function classificationLabel(value: string | undefined | null): string {
  if (!value) return CLASSIFICATIONS[1].label;
  const hit = CLASSIFICATIONS.find((c) => c.value === value.toLowerCase());
  return hit?.label ?? value;
}

export function shareModeLabel(value: string | undefined | null): string {
  if (!value) return SHARE_MODES[0].label;
  const hit = SHARE_MODES.find((m) => m.value === value);
  return hit?.label ?? value;
}

/** Badge variant SSOT — 005-front-designer: Public outline / Internal secondary / Confidential default / Secret destructive. */
export type SecurityBadgeVariant =
  | "outline"
  | "secondary"
  | "default"
  | "destructive";

export function classificationBadgeVariant(
  value: string | undefined | null,
): SecurityBadgeVariant {
  switch ((value ?? "internal").toLowerCase()) {
    case "public":
      return "outline";
    case "internal":
      return "secondary";
    case "confidential":
      return "default";
    case "secret":
      return "destructive";
    default:
      return "secondary";
  }
}

export function shareModeBadgeVariant(
  value: string | undefined | null,
): SecurityBadgeVariant {
  switch (value) {
    case "owner_only":
      return "destructive";
    case "classified":
      return "default";
    case "acl":
      return "secondary";
    default:
      return "outline";
  }
}

/** Flatten SecurityFields into a plain record for metadata merge / FormData. */
export function securityFieldsToRecord(
  fields: SecurityFields,
): Record<string, string | boolean> {
  const out: Record<string, string | boolean> = {
    classification: fields.classification,
    share_mode: fields.share_mode,
    export_control: fields.export_control,
    pii: fields.pii,
  };
  if (fields.project_id?.trim()) {
    out.project_id = fields.project_id.trim();
  }
  if (fields.security_status) {
    out.security_status = fields.security_status;
  }
  if (
    fields.share_mode === "acl" &&
    fields.acl_principal_ids &&
    fields.acl_principal_ids.length > 0
  ) {
    out.acl_principal_ids = fields.acl_principal_ids.join(",");
  }
  return out;
}
