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
import { Skeleton } from '@/components/ui/skeleton';
import { apiClient } from '@/lib/api/client';
import { getTenants } from '@/lib/api/edgequake/workspaces';
import { useAuthStore } from '@/stores/use-auth-store';
import type { Tenant } from '@/types';
import { Shield } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';

interface TenantQuotaRow extends Tenant {
  current_workspace_count?: number;
}

export function AdminQuotaSection() {
  const currentUser = useAuthStore((s) => s.user);
  const hasHydrated = useAuthStore((s) => s._hasHydrated);
  const isAdmin =
    currentUser?.role === 'admin' ||
    currentUser?.roles?.includes('admin') ||
    false;

  const [tenants, setTenants] = useState<TenantQuotaRow[]>([]);
  const [serverDefault, setServerDefault] = useState<number | null>(null);
  const [newDefault, setNewDefault] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSavingDefault, setIsSavingDefault] = useState(false);
  const [editingTenant, setEditingTenant] = useState<TenantQuotaRow | null>(null);

  // Load tenants and server default on mount (admin only)
  useEffect(() => {
    if (!hasHydrated || !isAdmin) {
      setIsLoading(false);
      return;
    }
    async function load() {
      setIsLoading(true);
      try {
        const [tenantsData, defaultsData] = await Promise.all([
          getTenants(),
          apiClient<{ default_max_workspaces: number }>("/admin/config/defaults"),
        ]);
        setTenants(tenantsData);
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
  }, [hasHydrated, isAdmin]);

  // SPEC-100: skeleton while auth/admin check or data loads (never return null→tall)
  if (!hasHydrated || (isAdmin && isLoading)) {
    return (
      <Card data-testid="spec100-admin-quota-skeleton">
        <CardHeader className="pb-3">
          <Skeleton className="h-5 w-24" />
          <Skeleton className="h-3 w-64" />
        </CardHeader>
        <CardContent className="min-h-[12rem] space-y-3">
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-20 w-full" />
        </CardContent>
      </Card>
    );
  }

  if (!isAdmin) {
    return null;
  }

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

  return (
    <>
      <Card data-testid="spec100-admin-quota-section">
        <CardHeader className="pb-3">
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-muted-foreground" />
            <CardTitle>Admin</CardTitle>
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
              <span
                className="ml-1 font-normal normal-case tracking-normal"
                data-testid="admin-quota-tenant-count"
              >
                ({tenants.length})
              </span>
            </label>
            {tenants.length === 0 ? (
              <p className="text-xs text-muted-foreground">No tenants found.</p>
            ) : (
              <div className="space-y-1">
                {tenants.map((tenant) => (
                  <div
                    key={tenant.id}
                    data-testid={`admin-quota-tenant-${tenant.name}`}
                    className="flex items-center justify-between gap-2 rounded px-2 py-1.5 hover:bg-muted/50"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-xs font-medium truncate">{tenant.name}</span>
                      <Badge variant="outline" className="text-xs h-4 px-1 py-0 shrink-0">
                        {tenant.plan}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <span className="text-xs text-muted-foreground">
                        {tenant.current_workspace_count ?? '?'}/{tenant.max_workspaces}
                      </span>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 px-2 text-xs"
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
