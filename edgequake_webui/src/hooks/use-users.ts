/**
 * @module use-users
 * @description Custom hook encapsulating user management state and operations.
 *
 * Separates data/CRUD concerns from rendering (SRP).
 *
 * @implements Issue #205 – User management admin UI
 */

import {
  createUser,
  deleteUser,
  listUsers,
  updateUser,
  type CreateUserRequest,
  type UpdateUserRequest,
  type UserInfo,
} from '@/lib/api/users';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Extracts a human-readable message from an unknown error (DRY). */
export const errMsg = (e: unknown): string =>
  e instanceof Error ? e.message : 'Unknown error';

// ── Hook ─────────────────────────────────────────────────────────────────────

export interface UseUsersReturn {
  users: UserInfo[];
  total: number;
  page: number;
  totalPages: number;
  isLoading: boolean;
  load: (p?: number) => Promise<void>;
  handleCreate: (data: CreateUserRequest) => Promise<boolean>;
  handleRoleChange: (userId: string, role: string) => Promise<void>;
  handleToggleActive: (userId: string, isActive: boolean) => Promise<void>;
  handleDelete: (userId: string, username: string) => Promise<void>;
}

export function useUsers(): UseUsersReturn {
  const [users, setUsers] = useState<UserInfo[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [isLoading, setIsLoading] = useState(false);

  // ----- Load ---------------------------------------------------------------

  const load = useCallback(async (p = 1) => {
    setIsLoading(true);
    try {
      const res = await listUsers(p, 20);
      setUsers(res.users);
      setTotal(res.total);
      setTotalPages(res.total_pages ?? 1);
      setPage(res.page);
    } catch (e) {
      toast.error(`Failed to load users: ${errMsg(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    load(1);
  }, [load]);

  // ----- Create -------------------------------------------------------------

  /** @returns true on success so the caller can close any dialog. */
  const handleCreate = async (data: CreateUserRequest): Promise<boolean> => {
    try {
      await createUser(data);
      toast.success(`User "${data.username}" created`);
      await load(1);
      return true;
    } catch (e) {
      toast.error(`Failed to create user: ${errMsg(e)}`);
      return false;
    }
  };

  // ----- Update role --------------------------------------------------------

  const handleRoleChange = async (userId: string, role: string): Promise<void> => {
    try {
      await updateUser(userId, { role } satisfies UpdateUserRequest);
      setUsers((prev) => prev.map((u) => (u.user_id === userId ? { ...u, role } : u)));
      toast.success('Role updated');
    } catch (e) {
      toast.error(`Failed to update role: ${errMsg(e)}`);
    }
  };

  // ----- Toggle active ------------------------------------------------------

  const handleToggleActive = async (userId: string, isActive: boolean): Promise<void> => {
    try {
      await updateUser(userId, { is_active: isActive } satisfies UpdateUserRequest);
      setUsers((prev) =>
        prev.map((u) => (u.user_id === userId ? { ...u, is_active: isActive } : u)),
      );
      toast.success(isActive ? 'User activated' : 'User deactivated');
    } catch (e) {
      toast.error(`Failed to update status: ${errMsg(e)}`);
    }
  };

  // ----- Delete -------------------------------------------------------------

  const handleDelete = async (userId: string, username: string): Promise<void> => {
    try {
      await deleteUser(userId);
      toast.success(`User "${username}" deleted`);
      await load(page);
    } catch (e) {
      toast.error(`Failed to delete user: ${errMsg(e)}`);
    }
  };

  return {
    users,
    total,
    page,
    totalPages,
    isLoading,
    load,
    handleCreate,
    handleRoleChange,
    handleToggleActive,
    handleDelete,
  };
}
