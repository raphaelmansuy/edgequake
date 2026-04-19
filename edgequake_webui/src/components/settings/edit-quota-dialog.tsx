'use client';

/**
 * @module edit-quota-dialog
 * @description Dialog for updating a tenant's max_workspaces quota.
 *
 * Implements SPEC-0001: Tenant Workspace Limits (Issue #133)
 *
 * Enforces validation rules:
 * - Input min = current workspace count (can't go below usage)
 * - Input max = 10000 (sanity limit)
 * - Shows current usage prominently
 */

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { apiClient } from '@/lib/api/client';
import type { Tenant } from '@/types';
import { useState } from 'react';
import { toast } from 'sonner';

interface EditQuotaDialogProps {
  tenant: Tenant & { current_workspace_count?: number };
  open: boolean;
  onClose: () => void;
  onUpdated: (tenantId: string, newMax: number) => void;
}

export function EditQuotaDialog({
  tenant,
  open,
  onClose,
  onUpdated,
}: EditQuotaDialogProps) {
  const currentMax = tenant.max_workspaces ?? 10;
  const currentCount = tenant.current_workspace_count ?? 0;

  const [newMax, setNewMax] = useState(String(currentMax));
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async () => {
    const val = parseInt(newMax, 10);
    if (isNaN(val) || val <= 0) {
      toast.error('Please enter a valid positive number');
      return;
    }
    if (val > 10_000) {
      toast.error('Value exceeds maximum allowed (10000)');
      return;
    }
    if (val < currentCount) {
      toast.error(`Cannot reduce below current usage (${currentCount})`);
      return;
    }

    setIsSubmitting(true);
    try {
      const data = await apiClient<{
        previous_max_workspaces: number;
        max_workspaces: number;
      }>(`/admin/tenants/${tenant.id}/quota`, {
        method: 'PATCH',
        body: JSON.stringify({ max_workspaces: val }),
      });
      toast.success(
        `Quota updated: ${data.previous_max_workspaces} → ${data.max_workspaces} for ${tenant.name}`
      );
      onUpdated(tenant.id, data.max_workspaces);
      onClose();
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(`Failed to update quota: ${msg}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!v) onClose(); }}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle className="text-base">Edit Quota</DialogTitle>
        </DialogHeader>

        <div className="space-y-3 py-2">
          <p className="text-sm font-medium">
            {tenant.name}
            {tenant.plan && (
              <span className="ml-2 text-xs font-normal text-muted-foreground capitalize">
                · {tenant.plan}
              </span>
            )}
          </p>

          <div className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
            <span className="text-muted-foreground">Current usage</span>
            <span className="font-medium">{currentCount} workspaces</span>
            <span className="text-muted-foreground">Current max</span>
            <span className="font-medium">{currentMax}</span>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="new-max" className="text-xs">
              New max
            </Label>
            <Input
              id="new-max"
              type="number"
              min={currentCount}
              max={10000}
              value={newMax}
              onChange={(e) => setNewMax(e.target.value)}
              className="h-8 text-sm"
            />
            <p className="text-[11px] text-muted-foreground">
              Min: {currentCount} (in use) · Max: 10000
            </p>
          </div>
        </div>

        <DialogFooter className="gap-2">
          <Button variant="outline" size="sm" onClick={onClose} disabled={isSubmitting}>
            Cancel
          </Button>
          <Button size="sm" onClick={handleSubmit} disabled={isSubmitting}>
            {isSubmitting ? 'Updating…' : 'Update Quota'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
