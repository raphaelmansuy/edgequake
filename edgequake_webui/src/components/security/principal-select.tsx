'use client';

/**
 * SPEC-146 principal picker — one Select used by members, ACL, attributes, break-glass.
 */

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { UserInfo } from '@/lib/api/users';
import { useTranslation } from 'react-i18next';

export function principalDisplayName(user: UserInfo | undefined, id: string): string {
  if (user) return user.username || user.email || id.slice(0, 8);
  // Never present a raw UUID as the primary label — operator UX.
  if (id.length >= 32) return 'Unknown owner';
  return id;
}

export interface PrincipalSelectProps {
  users: UserInfo[];
  value?: string;
  onValueChange: (userId: string) => void;
  placeholder?: string;
  disabledIds?: string[];
  isLoading?: boolean;
  testId?: string;
  className?: string;
  /** Empty value for "add" pickers (ACL). Default uses value or first unused. */
  allowEmpty?: boolean;
}

export function PrincipalSelect({
  users,
  value,
  onValueChange,
  placeholder,
  disabledIds = [],
  isLoading = false,
  testId = 'spec146-principal-select',
  className,
  allowEmpty: _allowEmpty = false,
}: PrincipalSelectProps) {
  const { t } = useTranslation();

  return (
    <Select
      value={value || undefined}
      onValueChange={onValueChange}
      disabled={isLoading}
    >
      <SelectTrigger className={className ?? 'h-9'} data-testid={testId}>
        <SelectValue
          placeholder={
            isLoading
              ? t('security.loadingUsers', 'Loading…')
              : placeholder ?? t('security.selectUser', 'Select user')
          }
        />
      </SelectTrigger>
      <SelectContent>
        {users.map((u) => (
          <SelectItem
            key={u.user_id}
            value={u.user_id}
            disabled={disabledIds.includes(u.user_id)}
          >
            {principalDisplayName(u, u.user_id)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
