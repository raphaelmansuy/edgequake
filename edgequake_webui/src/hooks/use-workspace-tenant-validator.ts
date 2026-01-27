/**
 * @fileoverview Auto-validation hook for workspace-tenant consistency
 *
 * @implements FEAT0861 - Multi-tenancy with workspace isolation
 * @implements FEAT0862 - Tenant context persisted across sessions
 *
 * @enforces BR0504 - All API calls include tenant/workspace headers
 * @enforces BR0861 - Workspace must belong to selected tenant
 *
 * WHY: Prevents tenant/workspace context mismatch issues
 *
 * This hook automatically detects and fixes scenarios where:
 * - LocalStorage has corrupted tenant/workspace pairing
 * - User switches tenants but old workspace ID persists
 * - Multiple tenants have identically-named workspaces
 *
 * Example scenario it prevents:
 * - Tenant A has "Default Workspace" (ID: xxx) with 13 entities
 * - Tenant B has "Default Workspace" (ID: yyy) with 0 entities
 * - User switches to Tenant A but localStorage still has workspace yyy
 * - Dashboard shows 0 entities while UI displays "Default Workspace"
 * - This hook detects the mismatch and auto-corrects to workspace xxx
 */

"use client";

import { getWorkspace } from "@/lib/api/edgequake";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";

interface ValidationResult {
  isValid: boolean;
  reason?: string;
  correctedWorkspaceId?: string;
}

/**
 * Hook that validates and auto-corrects workspace-tenant consistency.
 *
 * Call this hook in any component that displays workspace-dependent data
 * (Dashboard, Workspace page, Query page, etc.)
 *
 * @param options.onValidationFailed - Callback when validation fails (optional)
 * @param options.autoCorrect - Whether to auto-correct mismatches (default: true)
 */
export function useWorkspaceTenantValidator(options?: {
  onValidationFailed?: (result: ValidationResult) => void;
  autoCorrect?: boolean;
}) {
  const {
    selectedTenantId,
    selectedWorkspaceId,
    workspaces,
    selectWorkspace,
    reset,
  } = useTenantStore();
  const queryClient = useQueryClient();
  const hasValidated = useRef(false);
  const autoCorrect = options?.autoCorrect ?? true;

  useEffect(() => {
    // Only validate once per mount
    if (hasValidated.current) return;

    // Skip if no context selected yet
    if (!selectedTenantId || !selectedWorkspaceId) return;

    const validateAndCorrect = async () => {
      hasValidated.current = true;

      try {
        // Step 1: Check if workspace exists in current tenant's workspace list
        const workspaceInList = workspaces.find(
          (w) => w.id === selectedWorkspaceId,
        );

        if (workspaceInList) {
          // Workspace is in the list, check tenant_id matches
          if (workspaceInList.tenant_id !== selectedTenantId) {
            const result: ValidationResult = {
              isValid: false,
              reason: `Workspace ${selectedWorkspaceId} belongs to tenant ${workspaceInList.tenant_id}, not ${selectedTenantId}`,
            };

            console.error(
              "[WorkspaceTenantValidator] Mismatch detected:",
              result.reason,
            );
            options?.onValidationFailed?.(result);

            if (autoCorrect) {
              // Auto-correct: Select first workspace from current tenant
              const firstWorkspace = workspaces.find(
                (w) => w.tenant_id === selectedTenantId,
              );
              if (firstWorkspace) {
                // Invalidate queries for OLD workspace before switching
                queryClient.invalidateQueries({
                  queryKey: ["workspaceStats", selectedWorkspaceId],
                });
                queryClient.invalidateQueries({ queryKey: ["documents"] });
                queryClient.invalidateQueries({ queryKey: ["graph"] });

                // Switch to new workspace
                selectWorkspace(firstWorkspace.id);

                // Invalidate queries for NEW workspace to trigger fresh fetch
                queryClient.invalidateQueries({
                  queryKey: ["workspaceStats", firstWorkspace.id],
                });
                return;
              } else {
                // No valid workspace found, reset context
                reset();
                return;
              }
            }
          }
          // Valid - workspace is in list and tenant matches
          return;
        }

        // Step 2: Workspace not in list, fetch from API to verify
        const workspace = await getWorkspace(
          selectedTenantId,
          selectedWorkspaceId,
        );

        if (workspace.tenant_id !== selectedTenantId) {
          const result: ValidationResult = {
            isValid: false,
            reason: `Workspace ${selectedWorkspaceId} (${workspace.name}) belongs to tenant ${workspace.tenant_id}, not ${selectedTenantId}`,
          };

          console.error(
            "[WorkspaceTenantValidator] API verification failed:",
            result.reason,
          );
          options?.onValidationFailed?.(result);

          if (autoCorrect) {
            // Find a valid workspace for this tenant
            const validWorkspace = workspaces.find(
              (w) => w.tenant_id === selectedTenantId,
            );
            if (validWorkspace) {
              // Invalidate queries for OLD workspace before switching
              queryClient.invalidateQueries({
                queryKey: ["workspaceStats", selectedWorkspaceId],
              });
              queryClient.invalidateQueries({ queryKey: ["documents"] });
              queryClient.invalidateQueries({ queryKey: ["graph"] });

              // Switch to new workspace
              selectWorkspace(validWorkspace.id);

              // Invalidate queries for NEW workspace to trigger fresh fetch
              queryClient.invalidateQueries({
                queryKey: ["workspaceStats", validWorkspace.id],
              });
            } else {
              reset();
            }
          }
        }
      } catch (error) {
        // Workspace doesn't exist or API error
        console.error("[WorkspaceTenantValidator] Validation error:", error);

        if (autoCorrect) {
          // Select first available workspace for current tenant
          const firstWorkspace = workspaces.find(
            (w) => w.tenant_id === selectedTenantId,
          );
          if (firstWorkspace) {
            // Invalidate queries for OLD workspace before switching
            queryClient.invalidateQueries({
              queryKey: ["workspaceStats", selectedWorkspaceId],
            });
            queryClient.invalidateQueries({ queryKey: ["documents"] });
            queryClient.invalidateQueries({ queryKey: ["graph"] });

            // Switch to new workspace
            selectWorkspace(firstWorkspace.id);

            // Invalidate queries for NEW workspace to trigger fresh fetch
            queryClient.invalidateQueries({
              queryKey: ["workspaceStats", firstWorkspace.id],
            });
          } else {
            reset();
          }
        }
      }
    };

    validateAndCorrect();
  }, [
    selectedTenantId,
    selectedWorkspaceId,
    workspaces,
    selectWorkspace,
    reset,
    queryClient,
    autoCorrect,
    options,
  ]);
}
