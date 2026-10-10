'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { apiClient } from '@/lib/api/client';
import { updateWorkspace } from '@/lib/api/edgequake/workspaces';
import { useTenantStore } from '@/stores/use-tenant-store';
import { Grid3x3 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';

const ROLES = ['extract', 'query', 'keyword', 'summary', 'vlm', 'embedding'] as const;

type RoleKey = (typeof ROLES)[number];

interface ConnectionView {
  id: string;
  slug: string;
  display_name: string;
  source: string;
}

interface RoleRow {
  provider: string;
  model: string;
  connection_id: string;
}

const emptyRow = (): RoleRow => ({ provider: '', model: '', connection_id: '' });

export function RoleMatrixCard() {
  const selectedTenantId = useTenantStore((s) => s.selectedTenantId);
  const selectedWorkspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const tenants = useTenantStore((s) => s.tenants);
  const workspaces = useTenantStore((s) => s.workspaces);
  const tenant = tenants.find((t) => t.id === selectedTenantId);
  const workspace = workspaces.find((w) => w.id === selectedWorkspaceId);
  const [rows, setRows] = useState<Record<RoleKey, RoleRow>>(() => {
    const init = {} as Record<RoleKey, RoleRow>;
    for (const r of ROLES) init[r] = emptyRow();
    return init;
  });
  const [connections, setConnections] = useState<ConnectionView[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void apiClient<ConnectionView[]>('/connections')
      .then((data) => setConnections(Array.isArray(data) ? data : []))
      .catch(() => setConnections([]));
  }, []);

  useEffect(() => {
    const roles = workspace?.llm_roles ?? {};
    setRows((prev) => {
      const next = { ...prev };
      for (const r of ROLES) {
        const cfg = roles[r];
        next[r] = {
          provider: cfg?.provider ?? '',
          model: cfg?.model ?? '',
          connection_id: (cfg as { connection_id?: string } | undefined)?.connection_id ?? '',
        };
      }
      return next;
    });
  }, [workspace]);

  const save = useCallback(async () => {
    if (!workspace || !tenant) {
      toast.error('Select a workspace first');
      return;
    }
    setBusy(true);
    try {
      const llm_roles: Record<string, { provider?: string; model?: string; connection_id?: string }> =
        {};
      for (const r of ROLES) {
        const row = rows[r];
        if (!row.provider && !row.model && !row.connection_id) continue;
        llm_roles[r] = {
          provider: row.provider || undefined,
          model: row.model || undefined,
          connection_id: row.connection_id || undefined,
        };
      }
      await updateWorkspace(tenant.id, workspace.id, { llm_roles });
      toast.success('Role matrix saved');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  }, [rows, tenant, workspace]);

  return (
    <Card data-testid="role-matrix-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Grid3x3 className="h-5 w-5" />
          Workspace role matrix
        </CardTitle>
        <CardDescription>
          Each ingest/query role can use a saved Connection plus a model id. Empty cells inherit
          the workspace default.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-muted-foreground">
                <th className="py-1 pr-2">Role</th>
                <th className="py-1 pr-2">Connection</th>
                <th className="py-1 pr-2">Provider</th>
                <th className="py-1">Model</th>
              </tr>
            </thead>
            <tbody>
              {ROLES.map((role) => (
                <tr key={role}>
                  <td className="py-1 pr-2 font-medium">{role}</td>
                  <td className="py-1 pr-2">
                    <select
                      className="w-full rounded-md border bg-background px-2 py-1"
                      value={rows[role].connection_id}
                      onChange={(e) =>
                        setRows((s) => ({
                          ...s,
                          [role]: { ...s[role], connection_id: e.target.value },
                        }))
                      }
                    >
                      <option value="">(inherit / env)</option>
                      {connections.map((c) => (
                        <option key={`${c.source}-${c.id}-${c.slug}`} value={c.id === '00000000-0000-0000-0000-000000000000' ? '' : c.id}>
                          {c.display_name}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="py-1 pr-2">
                    <Input
                      value={rows[role].provider}
                      onChange={(e) =>
                        setRows((s) => ({
                          ...s,
                          [role]: { ...s[role], provider: e.target.value },
                        }))
                      }
                      placeholder="ollama"
                    />
                  </td>
                  <td className="py-1">
                    <Input
                      value={rows[role].model}
                      onChange={(e) =>
                        setRows((s) => ({
                          ...s,
                          [role]: { ...s[role], model: e.target.value },
                        }))
                      }
                      placeholder="model id"
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <Button type="button" disabled={busy || !workspace} onClick={() => void save()}>
          Save role matrix
        </Button>
      </CardContent>
    </Card>
  );
}
