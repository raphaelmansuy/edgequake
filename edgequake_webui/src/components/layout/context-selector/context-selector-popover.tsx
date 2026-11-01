'use client';

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command';
import { smartTruncate } from '@/lib/layout/format-context-labels';
import { cn } from '@/lib/utils';
import type { Tenant, Workspace } from '@/types';
import { Building2, Check, FolderKanban, Plus } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

export interface ContextSelectorPopoverProps {
  tenants: Tenant[];
  workspaces: Workspace[];
  selectedTenantId: string | null;
  selectedWorkspaceId: string | null;
  selectedTenantName?: string | null;
  selectedWorkspaceName?: string | null;
  isLoadingWorkspaces?: boolean;
  isWorkspacesError?: boolean;
  /** Remount cmdk when the popover opens so leftover search cannot hide rows. */
  searchResetKey?: string | number;
  onTenantSelect: (tenantId: string) => void;
  onWorkspaceSelect: (workspaceId: string) => void;
  onCreateTenant: () => void;
  onCreateWorkspace: () => void;
  /** SPEC-181 LAW-181-3: a create row is rendered only when the server would accept it. */
  canCreateTenant?: boolean;
  canCreateWorkspace?: boolean;
  /** At the plan limit the row stays, disabled, with the fix. */
  quotaBlockedWorkspace?: boolean;
  workspaceQuota?: { used: number; limit: number };
  /** Effective role per workspace id, shown as a chip so "what am I here?" needs no page load. */
  workspaceRoles?: Record<string, string>;
}

/**
 * SPEC-101 LAW-101-11 — Select Organization (tenant), then Workspace.
 * Order: 1 Organizations → 2 Workspaces. Tenant select keeps popover open.
 */
export function ContextSelectorPopover({
  tenants,
  workspaces,
  selectedTenantId,
  selectedWorkspaceId,
  selectedTenantName,
  isLoadingWorkspaces,
  isWorkspacesError,
  searchResetKey = 0,
  onTenantSelect,
  onWorkspaceSelect,
  onCreateTenant,
  onCreateWorkspace,
  canCreateTenant = false,
  canCreateWorkspace = false,
  quotaBlockedWorkspace = false,
  workspaceQuota,
  workspaceRoles,
}: ContextSelectorPopoverProps) {
  const { t } = useTranslation();
  const workspacesRef = useRef<HTMLDivElement>(null);
  const prevTenantRef = useRef<string | null>(null);

  // After tenant change, scroll Workspaces into view (step 2).
  useEffect(() => {
    if (!selectedTenantId) return;
    if (prevTenantRef.current && prevTenantRef.current !== selectedTenantId) {
      workspacesRef.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }
    prevTenantRef.current = selectedTenantId;
  }, [selectedTenantId]);

  const tenantHint = selectedTenantName
    ? smartTruncate(selectedTenantName, 24)
    : null;

  return (
    <Command key={searchResetKey}>
      <CommandInput
        placeholder={t(
          'context.searchPlaceholder',
          'Search organizations and workspaces...',
        )}
        className="h-9"
        data-testid="context-selector-search"
      />
      <CommandList className="max-h-[min(24rem,70vh)]">
        <CommandEmpty>{t('context.noMatches', 'No matches')}</CommandEmpty>

        <div data-testid="context-selector-tenants">
          <CommandGroup
            heading={t('context.stepOrganization', '1 · Organization')}
          >
                {tenants.map((tenant) => {
              const selected = tenant.id === selectedTenantId;
              return (
                <CommandItem
                  key={tenant.id}
                  value={tenant.id}
                  keywords={[tenant.name, tenant.slug ?? '']}
                  title={tenant.name}
                  data-testid={`tenant-option-${tenant.slug ?? tenant.id}`}
                  onSelect={() => onTenantSelect(tenant.id)}
                  className={cn(selected && 'bg-accent')}
                >
                  <Building2
                    className="mr-2 h-4 w-4 shrink-0 text-muted-foreground"
                    aria-hidden="true"
                  />
                  <span className="min-w-0 flex-1 truncate text-sm">{tenant.name}</span>
                  {selected ? <Check className="ml-2 h-4 w-4 shrink-0 text-primary" /> : null}
                </CommandItem>
              );
            })}
            {canCreateTenant ? (
              <CommandItem
                value="create-organization"
                onSelect={onCreateTenant}
                data-testid="header-create-tenant"
              >
                <Plus className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                {t('tenant.createNew', 'New organization')}
              </CommandItem>
            ) : null}
          </CommandGroup>
        </div>

        {selectedTenantId ? (
          <>
            <CommandSeparator />
            <div ref={workspacesRef} data-testid="context-selector-workspaces">
              <CommandGroup
                heading={
                  tenantHint
                    ? t('context.stepWorkspaceInCount', '2 · Workspace · {{tenant}} ({{count}})', {
                        tenant: tenantHint,
                        count: workspaces.length,
                      })
                    : t('context.stepWorkspaceCount', '2 · Workspace ({{count}})', {
                        count: workspaces.length,
                      })
                }
              >
                {isWorkspacesError ? (
                  <div className="px-3 py-2 text-xs text-destructive">
                    {t(
                      'tenant.workspaceLoadErrorShort',
                      'Could not load workspaces for this organization.',
                    )}
                  </div>
                ) : null}
                {workspaces.length === 0 && !isLoadingWorkspaces && !isWorkspacesError ? (
                  <div className="px-3 py-2 text-xs text-muted-foreground">
                    {t('workspace.empty', 'No workspaces yet')}
                  </div>
                ) : null}
                {isLoadingWorkspaces && workspaces.length === 0 ? (
                  <div className="px-3 py-2 text-xs text-muted-foreground">
                    {t('common.loading', 'Loading...')}
                  </div>
                ) : null}
                {workspaces.map((workspace) => {
                  const selected = workspace.id === selectedWorkspaceId;
                  const optionId = workspace.slug || workspace.id;
                  return (
                    <CommandItem
                      key={workspace.id}
                      value={workspace.id}
                      keywords={[workspace.name, workspace.slug ?? '']}
                      title={workspace.name}
                      data-testid={`workspace-option-${optionId}`}
                      onSelect={() => onWorkspaceSelect(workspace.id)}
                      className={cn(selected && 'bg-accent')}
                    >
                      <FolderKanban
                        className="mr-2 h-4 w-4 shrink-0 text-muted-foreground"
                        aria-hidden="true"
                      />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium">{workspace.name}</div>
                        {workspace.slug ? (
                          <div className="truncate font-mono text-xs text-muted-foreground">
                            {workspace.slug}
                          </div>
                        ) : null}
                      </div>
                      {workspaceRoles?.[workspace.id] ? (
                        <span
                          className="ml-2 shrink-0 rounded-full bg-muted px-1.5 py-0.5 text-[10.5px] font-medium capitalize text-muted-foreground"
                          data-testid={`workspace-role-${workspace.slug ?? workspace.id}`}
                        >
                          {workspaceRoles[workspace.id]}
                        </span>
                      ) : null}
                      {selected ? (
                        <Check className="ml-2 h-4 w-4 shrink-0 text-primary" />
                      ) : null}
                    </CommandItem>
                  );
                })}
                {canCreateWorkspace ? (
                  <CommandItem
                    value="create-workspace"
                    onSelect={onCreateWorkspace}
                    data-testid="header-create-workspace"
                  >
                    <Plus className="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
                    {t('workspace.createNew', 'New workspace')}
                  </CommandItem>
                ) : quotaBlockedWorkspace ? (
                  <div
                    className="px-3 py-2 text-xs text-amber-700 dark:text-amber-400"
                    data-testid="header-create-workspace-blocked"
                  >
                    {t(
                      'workspace.quotaBlocked',
                      'Your plan allows {{limit}} workspaces. Archive one to free a slot.',
                      { limit: workspaceQuota?.limit ?? 0 },
                    )}
                  </div>
                ) : (
                  <div
                    className="px-3 py-2 text-xs text-muted-foreground"
                    data-testid="header-create-workspace-hint"
                  >
                    {t('workspace.askAdmin', 'Ask an admin to create a workspace.')}
                  </div>
                )}
              </CommandGroup>
            </div>
          </>
        ) : (
          <div
            className="border-t px-3 py-2 text-xs text-muted-foreground"
            data-testid="context-selector-pick-org-hint"
          >
            {t(
              'context.pickOrganizationFirst',
              'Select an organization to choose a workspace.',
            )}
          </div>
        )}
      </CommandList>
    </Command>
  );
}
