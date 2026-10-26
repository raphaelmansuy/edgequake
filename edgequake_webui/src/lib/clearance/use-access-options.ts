'use client';

import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useAuthStore } from "@/stores/use-auth-store";
import { accessApi } from './access-api';

export function useAccessOptions(workspace: string | null | undefined) {
  const tenant = useTenantStore(state => state.selectedTenantId);
  const principal = useAuthStore(state => state.user?.user_id ?? state.user?.id);
  return useQuery({
    queryKey: ['document-access-options', tenant, workspace, principal],
    queryFn: () => accessApi.options(workspace!), enabled: Boolean(workspace), retry: false,
    staleTime: 0,
  });
}
export function useSecurityTags(workspace: string | null | undefined, search: string, enabled: boolean) {
  const tenant = useTenantStore(state => state.selectedTenantId);
  const principal = useAuthStore(state => state.user?.user_id ?? state.user?.id);
  return useInfiniteQuery({
    queryKey: ['document-access-tags', tenant, workspace, principal, search],
    queryFn: ({ pageParam }) => accessApi.options(workspace!, search, pageParam),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: page => page.next_cursor ?? undefined,
    enabled: Boolean(workspace) && enabled, retry: false,
  });
}
