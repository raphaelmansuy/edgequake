"use client";

/**
 * SPEC-175 LAW-175-17 / SPEC-181 — the one badge that says what the caller may do here.
 *
 * The chip is derived from the caller's highest grant, in the order Platform admin → Super admin
 * → Workspace admin → Member. SPEC-181 adds the *organization role* so the badge can say
 * "Owner" or "Admin" when that is what the person actually is in the selected organization: a
 * badge that says "Super admin" to a customer's owner is jargon, and a badge that says
 * "Platform admin" to them would be wrong.
 */

import { Badge } from "@/components/ui/badge";
import { ACCESS_CHIP_COPY, accessChip, type SettingsAccess } from "@/lib/settings/access";
import { useAccess } from "@/lib/access/use-access";
import { useTranslation } from "react-i18next";

export function AccessBadge({ access }: { access: SettingsAccess }) {
  const { t } = useTranslation();
  const { access: me } = useAccess();

  // Platform admin never reads as anything else (LAW-175-17).
  if (me.platform.admin) {
    return <Chip label={t("settings.access.platformAdmin", "Platform admin")} chip="platform_admin" />;
  }

  // Organization role is the most useful thing to show next to an organization's settings.
  const orgRole = me.organization?.role;
  if (orgRole === "owner" || orgRole === "admin") {
    return (
      <Chip
        label={t(`members.roles.${orgRole}.label`, orgRole === "owner" ? "Owner" : "Admin")}
        chip={`org_${orgRole}`}
      />
    );
  }

  const chip = accessChip(access);
  const copy = ACCESS_CHIP_COPY[chip];
  return <Chip label={t(copy.label, copy.fallback)} chip={chip} />;
}

function Chip({ label, chip }: { label: string; chip: string }) {
  return (
    <Badge
      variant="outline"
      className="shrink-0 text-[11px] font-medium capitalize"
      data-testid="settings-access-badge"
      data-chip={chip}
    >
      {label}
    </Badge>
  );
}
