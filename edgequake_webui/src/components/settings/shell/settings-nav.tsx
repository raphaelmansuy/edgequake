"use client";

/**
 * SPEC-175 — the scope nav. Groups follow LAW-175-8: You, Workspace, Organization, Deployment.
 * Only sections the caller may open are rendered (EC-175-34, EC-175-43).
 *
 * Keyboard: Tab enters the nav; ArrowUp/ArrowDown, Home and End move between links (roving
 * focus, one stop per link in the Tab order is kept by the browser's native link focus).
 */

import { WorkspaceSwitcher } from "@/components/settings/shell/workspace-switcher";
import { nextNavIndex } from "@/lib/settings/keyboard";
import { cn } from "@/lib/utils";
import {
  sectionHref,
  visibleScopes,
  visibleSections,
  type SettingsSectionId,
} from "@/lib/settings/registry";
import type { SettingsAccess } from "@/lib/settings/access";
import type { Workspace } from "@/types";
import { Activity, Building2, Cpu, FileText, FolderKanban, HardDrive, KeyRound, Layers, Network, Paintbrush, Palette, Plug, Server, ShieldCheck, Users } from "lucide-react";
import Link from "next/link";
import { useId, type KeyboardEvent, type ComponentType } from "react";

const ICONS: Record<SettingsSectionId, ComponentType<{ className?: string }>> = {
  appearance: Palette,
  graph: Network,
  "browser-data": HardDrive,
  models: Cpu,
  ingestion: FileText,
  access: ShieldCheck,
  people: Users,
  workspaces: FolderKanban,
  branding: Paintbrush,
  "org-providers": Layers,
  secrets: KeyRound,
  connections: Plug,
  providers: Layers,
  fleet: Server,
  observability: Activity,
  organizations: Building2,
};

export function SettingsNav({
  access,
  activeSection,
  workspaceSlug,
  workspace,
  workspaces,
}: {
  access: SettingsAccess;
  activeSection: SettingsSectionId;
  workspaceSlug: string | null;
  workspace: Workspace | null;
  workspaces: readonly Workspace[];
}) {
  const navId = useId();

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    const links = Array.from(event.currentTarget.querySelectorAll<HTMLAnchorElement>("a[href]"));
    const current = links.indexOf(document.activeElement as HTMLAnchorElement);
    const next = nextNavIndex(current, links.length, event.key);
    if (next === null) return;
    event.preventDefault();
    links[next]?.focus();
  };

  return (
    <nav
      id={navId}
      aria-label="Settings sections"
      onKeyDown={onKeyDown}
      data-testid="settings-nav"
      className="flex flex-col gap-5"
    >
      {visibleScopes(access).map((scope) => {
        const sections = visibleSections(access, scope.id);
        const headingId = `${navId}-${scope.id}`;
        const groupName = scope.id === "workspace" && workspace ? workspace.name : null;
        return (
          <div key={scope.id} role="group" aria-labelledby={headingId} data-scope={scope.id}>
            <p
              id={headingId}
              className="flex min-w-0 items-baseline gap-1.5 px-2 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              <span className="shrink-0">{scope.fallback}</span>
              {groupName ? (
                <span className="min-w-0 truncate normal-case tracking-normal text-foreground/70" title={groupName}>
                  {groupName}
                </span>
              ) : null}
            </p>
            <ul className="flex flex-col gap-0.5">
              {sections.map((section) => {
                const Icon = ICONS[section.id];
                const current = section.id === activeSection;
                return (
                  <li key={section.id}>
                    <Link
                      href={sectionHref(section.id, workspaceSlug)}
                      aria-current={current ? "page" : undefined}
                      data-testid={`settings-nav-${section.id}`}
                      className={cn(
                        "flex h-8 items-center gap-2.5 rounded-md px-2 text-sm font-medium transition-colors",
                        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                        current
                          ? "bg-accent text-accent-foreground"
                          : "text-muted-foreground hover:bg-muted hover:text-foreground",
                      )}
                    >
                      {Icon ? <Icon className="h-4 w-4 shrink-0" aria-hidden="true" /> : null}
                      <span className="truncate">{section.label}</span>
                    </Link>
                  </li>
                );
              })}
            </ul>
            {scope.id === "workspace" ? (
              <WorkspaceSwitcher
                workspace={workspace}
                workspaces={workspaces}
                activeSection={activeSection}
                workspaceSlug={workspaceSlug}
              />
            ) : null}
          </div>
        );
      })}
    </nav>
  );
}
