'use client';

/**
 * @module admin-quota-section
 * @description Admin-only section in Settings for managing tenant workspace quotas
 * and server-wide defaults.
 *
 * Implements SPEC-0001: Tenant Workspace Limits (Issue #133)
 *
 * Only rendered when user.role === "admin". Non-admin users will not see this section.
 */

import { EditQuotaDialog } from '@/components/settings/edit-quota-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { apiClient } from '@/lib/api/client';
import type { Tenant } from '@/types';
import { Shield } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';

interface TenantQuotaRow extends Tenant {
  current_workspace_count?: number;
}

export function AdminQuotaSection() {
  const [tenants, setTenants] = useState<TenantQuotaRow[]>([]);
  const [serverDefault, setServerDefault] = useState<number | null>(null);
  const [newDefault, setNewDefault] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingDefault, setIsSavingDefault] = useState(false);
  const [editingTenant, setEditingTenant] = useState<TenantQuotaRow | null>(null);

  // Load tenants and server default on mount
  useEffect(() => {
    async function load() {
      setIsLoading(true);
      try {
        const [tenantsData, defaultsData] = await Promise.all([
          apiClient<{ items?: TenantQuotaRow[] }>("/tenants?limit=100"),
          apiClient<{ default_max_workspaces: number }>("/admin/config/defaults"),
        ]);
        setTenants(tenantsData.items ?? []);
        setServerDefault(defaultsData.default_max_workspaces);
        setNewDefault(String(defaultsData.default_max_workspaces));
      } catch (e) {
        // Ignore load errors; section is best-effort
        console.warn('AdminQuotaSection: failed to load data', e);
      } finally {
        setIsLoading(false);
      }
    }
    load();
  }, []);

  const handleSaveDefault = async () => {
    const val = parseInt(newDefault, 10);
    if (isNaN(val) || val <= 0) {
      toast.error('Please enter a valid positive number');
      return;
    }
    setIsSavingDefault(true);
    try {
      const data = await apiClient<{ default_max_workspaces: number }>(
        '/admin/config/defaults',
        {
          method: 'PATCH',
          body: JSON.stringify({ default_max_workspaces: val }),
        }
      );
      setServerDefault(data.default_max_workspaces);
      toast.success(`Server default updated to ${data.default_max_workspaces} workspaces`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(`Failed to update server default: ${msg}`);
    } finally {
      setIsSavingDefault(false);
    }
  };

  const handleQuotaUpdated = (tenantId: string, newMax: number) => {
    setTenants((prev) =>
      prev.map((t) =>
        t.id === tenantId ? { ...t, max_workspaces: newMax } : t
      )
    );
  };

  if (isLoading) {
    return null; // Don't flash admin section while loading
  }

  return (
    <>
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-muted-foreground" />
            <CardTitle className="text-base">Admin</CardTitle>
          </div>
          <CardDescription className="text-xs">
            Manage tenant workspace quotas and server-wide defaults. Admin only.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Server-wide default */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Server Default
            </label>
            <p className="text-xs text-muted-foreground">
              Max workspaces for newly created tenants.{' '}
              {serverDefault !== null && (
                <span>Current: <strong>{serverDefault}</strong></span>
              )}
            </p>
            <div className="flex items-center gap-2">
              <Input
                type="number"
                min={1}
                max={10000}
                value={newDefault}
                onChange={(e) => setNewDefault(e.target.value)}
                className="h-8 w-28 text-xs"
                placeholder="100"
              />
              <Button
                size="sm"
                variant="outline"
                className="h-8 text-xs"
                onClick={handleSaveDefault}
                disabled={isSavingDefault}
              >
                Save
              </Button>
            </div>
          </div>

          <div className="h-px bg-border" />

          {/* Tenant list */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Tenant Quotas
            </label>
            {tenants.length === 0 ? (
              <p className="text-xs text-muted-foreground">No tenants found.</p>
            ) : (
              <div className="space-y-1">
                {tenants.map((tenant) => (
                  <div
                    key={tenant.id}
                    className="flex items-center justify-between gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-[11px] font-medium truncate">{tenant.name}</span>
                      <Badge variant="outline" className="text-[10px] h-4 px-1 py-0 shrink-0">
                        {tenant.plan}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <span className="text-[11px] text-muted-foreground">
                        {tenant.current_workspace_count ?? '?'}/{tenant.max_workspaces}
                      </span>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 px-2 text-[11px]"
                        onClick={() => setEditingTenant(tenant)}
                      >
                        Edit
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Edit dialog (portal) */}
      {editingTenant && (
        <EditQuotaDialog
          tenant={editingTenant}
          open={!!editingTenant}
          onClose={() => setEditingTenant(null)}
          onUpdated={handleQuotaUpdated}
        />
      )}
    </>
  );
}
