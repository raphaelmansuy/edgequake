/**
 * @module users-api
 * @description User management API client.
 *
 * @implements Issue #205 – User management admin UI
 */

import { api } from './client';

// ============================================================================
// Types
// ============================================================================

export interface UserInfo {
  user_id: string;
  username: string;
  email: string;
  role: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  last_login_at?: string | null;
}

export interface ListUsersResponse {
  users: UserInfo[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface CreateUserRequest {
  username: string;
  email: string;
  password: string;
  role?: string;
}

export interface UpdateUserRequest {
  role?: string;
  is_active?: boolean;
  email?: string;
}

export interface CreateUserResponse {
  user: UserInfo;
  created_at: string;
}

export interface UpdateUserResponse {
  user: UserInfo;
  updated_at: string;
}

// ============================================================================
// API functions
// ============================================================================

export async function listUsers(
  page = 1,
  pageSize = 20,
  role?: string,
): Promise<ListUsersResponse> {
  const params = new URLSearchParams({ page: String(page), page_size: String(pageSize) });
  if (role) params.set('role', role);
  return api.get<ListUsersResponse>(`/users?${params}`);
}

export async function getUser(userId: string): Promise<UserInfo> {
  return api.get<UserInfo>(`/users/${userId}`);
}

export async function createUser(data: CreateUserRequest): Promise<CreateUserResponse> {
  return api.post<CreateUserResponse>('/users', data);
}

export async function updateUser(
  userId: string,
  data: UpdateUserRequest,
): Promise<UpdateUserResponse> {
  return api.patch<UpdateUserResponse>(`/users/${userId}`, data);
}

export async function deleteUser(userId: string): Promise<void> {
  return api.delete<void>(`/users/${userId}`);
}
