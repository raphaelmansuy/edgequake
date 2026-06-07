'use client';

/**
 * @module user-management-card
 * @description Admin-only card: list, create, update role/status, delete users.
 *
 * Pure UI — all data/CRUD delegated to `useUsers` hook (SRP).
 * WCAG AA: aria-labels, role="status", keyboard nav, focus trapping.
 *
 * @implements Issue #205 - User management admin UI
 */

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { useUsers } from '@/hooks/use-users';
import type { CreateUserRequest, UserInfo } from '@/lib/api/users';
import { useAuthStore } from '@/stores/use-auth-store';
import { Loader2, Plus, Shield, Trash2, UserCog } from 'lucide-react';
import { useState } from 'react';

// ── Constants ────────────────────────────────────────────────────────────────

const ROLES = ['admin', 'developer', 'viewer'] as const;
type Role = (typeof ROLES)[number];

const ROLE_BADGE_VARIANT: Record<Role, 'default' | 'secondary' | 'outline'> = {
  admin: 'default',
  developer: 'secondary',
  viewer: 'outline',
};

const EMPTY_FORM: CreateUserRequest = { username: '', email: '', password: '', role: 'viewer' };

// ── Root component ────────────────────────────────────────────────────────────

export function UserManagementCard() {
  const currentUser = useAuthStore((s) => s.user);
  const isAdmin = currentUser?.role === 'admin' || currentUser?.roles?.includes('admin') || false;

  const {
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
  } = useUsers();

  const [showCreate, setShowCreate] = useState(false);
  const [form, setForm] = useState<CreateUserRequest>(EMPTY_FORM);
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isAdmin) return null;

  const onSubmitCreate = async () => {
    const { username, email, password } = form;
    if (!username.trim() || !email.trim() || !password.trim()) return;
    setIsSubmitting(true);
    const ok = await handleCreate(form);
    setIsSubmitting(false);
    if (ok) {
      setShowCreate(false);
      setForm(EMPTY_FORM);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
            <CardTitle className="text-base">User Management</CardTitle>
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setShowCreate(true)}
            aria-label="Create new user"
          >
            <Plus className="h-3.5 w-3.5 mr-1" aria-hidden="true" />
            New User
          </Button>
        </div>
        <CardDescription className="text-xs">
          {total} user{total !== 1 ? 's' : ''}
        </CardDescription>
      </CardHeader>

      <CardContent className="px-0 pb-0">
        {isLoading ? (
          <LoadingState />
        ) : users.length === 0 ? (
          <EmptyState />
        ) : (
          <>
            <UsersTable
              users={users}
              onRoleChange={handleRoleChange}
              onToggleActive={handleToggleActive}
              onDelete={handleDelete}
            />
            {totalPages > 1 && (
              <PaginationBar page={page} totalPages={totalPages} onNavigate={load} />
            )}
          </>
        )}
      </CardContent>

      <CreateUserDialog
        open={showCreate}
        form={form}
        isSubmitting={isSubmitting}
        onFormChange={(patch) => setForm((f) => ({ ...f, ...patch }))}
        onSubmit={onSubmitCreate}
        onClose={() => {
          setShowCreate(false);
          setForm(EMPTY_FORM);
        }}
      />
    </Card>
  );
}

// ── Sub-components (pure, SRP) ────────────────────────────────────────────────

function LoadingState() {
  return (
    <div
      className="flex items-center justify-center py-10"
      role="status"
      aria-label="Loading users"
    >
      <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" aria-hidden="true" />
    </div>
  );
}

function EmptyState() {
  return (
    <p className="text-sm text-muted-foreground text-center py-8 px-4">No users found.</p>
  );
}

interface UsersTableProps {
  users: UserInfo[];
  onRoleChange: (userId: string, role: string) => void;
  onToggleActive: (userId: string, isActive: boolean) => void;
  onDelete: (userId: string, username: string) => void;
}

function UsersTable({ users, onRoleChange, onToggleActive, onDelete }: UsersTableProps) {
  return (
    <div className="border-t">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[130px] pl-4">Username</TableHead>
            <TableHead>Email</TableHead>
            <TableHead className="w-[110px]">Role</TableHead>
            <TableHead className="w-[70px] text-center">Active</TableHead>
            <TableHead className="w-[48px] pr-4">
              <span className="sr-only">Actions</span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {users.map((u) => (
            <TableRow key={u.user_id}>
              <TableCell className="font-medium pl-4">
                <span className="flex items-center gap-1.5 text-sm">
                  <UserCog className="h-3 w-3 text-muted-foreground shrink-0" aria-hidden="true" />
                  {u.username}
                </span>
              </TableCell>
              <TableCell className="text-xs text-muted-foreground truncate max-w-[180px]">
                {u.email}
              </TableCell>
              <TableCell>
                <Select
                  value={u.role}
                  onValueChange={(value) => onRoleChange(u.user_id, value)}
                  aria-label={`Role for ${u.username}`}
                >
                  <SelectTrigger className="h-6 text-xs w-[96px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLES.map((r) => (
                      <SelectItem key={r} value={r} className="text-xs">
                        <Badge
                          variant={ROLE_BADGE_VARIANT[r as Role]}
                          className="text-xs font-normal px-1.5 py-0"
                        >
                          {r}
                        </Badge>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </TableCell>
              <TableCell className="text-center">
                <Switch
                  checked={u.is_active}
                  onCheckedChange={(checked) => onToggleActive(u.user_id, checked)}
                  aria-label={
                    u.is_active ? `Deactivate ${u.username}` : `Activate ${u.username}`
                  }
                  className="scale-75 data-[state=checked]:bg-green-600"
                />
              </TableCell>
              <TableCell className="pr-4 text-right">
                <DeleteUserButton
                  userId={u.user_id}
                  username={u.username}
                  onDelete={onDelete}
                />
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

interface DeleteUserButtonProps {
  userId: string;
  username: string;
  onDelete: (userId: string, username: string) => void;
}

function DeleteUserButton({ userId, username, onDelete }: DeleteUserButtonProps) {
  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 text-muted-foreground hover:text-destructive"
          aria-label={`Delete user ${username}`}
        >
          <Trash2 className="h-3 w-3" aria-hidden="true" />
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete user?</AlertDialogTitle>
          <AlertDialogDescription>
            Permanently delete <strong>{username}</strong>. This cannot be undone.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={() => onDelete(userId, username)}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

interface PaginationBarProps {
  page: number;
  totalPages: number;
  onNavigate: (p: number) => void;
}

function PaginationBar({ page, totalPages, onNavigate }: PaginationBarProps) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-t">
      <p className="text-xs text-muted-foreground">
        Page {page} of {totalPages}
      </p>
      <div className="flex gap-1.5">
        <Button
          variant="outline"
          size="sm"
          className="h-7 text-xs"
          disabled={page <= 1}
          onClick={() => onNavigate(page - 1)}
          aria-label="Previous page"
        >
          Previous
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="h-7 text-xs"
          disabled={page >= totalPages}
          onClick={() => onNavigate(page + 1)}
          aria-label="Next page"
        >
          Next
        </Button>
      </div>
    </div>
  );
}

interface CreateUserDialogProps {
  open: boolean;
  form: CreateUserRequest;
  isSubmitting: boolean;
  onFormChange: (patch: Partial<CreateUserRequest>) => void;
  onSubmit: () => void;
  onClose: () => void;
}

function CreateUserDialog({
  open,
  form,
  isSubmitting,
  onFormChange,
  onSubmit,
  onClose,
}: CreateUserDialogProps) {
  const isValid = Boolean(form.username.trim() && form.email.trim() && form.password.trim());

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && isValid && !isSubmitting) {
      e.preventDefault();
      onSubmit();
    }
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent aria-describedby="create-user-desc" onKeyDown={onKeyDown}>
        <DialogHeader>
          <DialogTitle>New User</DialogTitle>
        </DialogHeader>
        <p id="create-user-desc" className="text-xs text-muted-foreground -mt-1">
          All fields are required.
        </p>
        <div className="grid gap-3 py-1">
          <FormField
            id="cu-username"
            label="Username"
            value={form.username}
            placeholder="alice"
            autoComplete="off"
            onChange={(v) => onFormChange({ username: v })}
          />
          <FormField
            id="cu-email"
            label="Email"
            type="email"
            value={form.email}
            placeholder="alice@example.com"
            autoComplete="off"
            onChange={(v) => onFormChange({ email: v })}
          />
          <FormField
            id="cu-password"
            label="Password"
            type="password"
            value={form.password}
            placeholder="..."
            autoComplete="new-password"
            onChange={(v) => onFormChange({ password: v })}
          />
          <div className="grid gap-1">
            <Label htmlFor="cu-role" className="text-xs">
              Role
            </Label>
            <Select value={form.role} onValueChange={(v) => onFormChange({ role: v })}>
              <SelectTrigger id="cu-role" className="h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ROLES.map((r) => (
                  <SelectItem key={r} value={r} className="text-sm">
                    {r.charAt(0).toUpperCase() + r.slice(1)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" onClick={onSubmit} disabled={!isValid || isSubmitting}>
            {isSubmitting && (
              <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" aria-hidden="true" />
            )}
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface FormFieldProps {
  id: string;
  label: string;
  value: string;
  placeholder?: string;
  type?: string;
  autoComplete?: string;
  onChange: (value: string) => void;
}

/** Reusable label + input row (DRY). */
function FormField({ id, label, value, placeholder, type = 'text', autoComplete, onChange }: FormFieldProps) {
  return (
    <div className="grid gap-1">
      <Label htmlFor={id} className="text-xs">
        {label}
      </Label>
      <Input
        id={id}
        type={type}
        value={value}
        placeholder={placeholder}
        autoComplete={autoComplete}
        className="h-8 text-sm"
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}
