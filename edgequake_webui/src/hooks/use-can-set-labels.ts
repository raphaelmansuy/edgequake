'use client';

/**
 * SPEC-146: document:set_labels capability (app RBAC).
 * Mirrors edgequake-auth: Admin + User yes; Readonly no.
 * When auth is off / no session user, allow (ambient operator / local mocks).
 */

import { useAuthStore } from '@/stores/use-auth-store';

export function useCanSetLabels(): boolean {
  const role = useAuthStore((s) => s.user?.role);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (!isAuthenticated || !role) return true;
  return role === 'admin' || role === 'user';
}
