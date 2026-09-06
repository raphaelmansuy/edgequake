'use client';

/**
 * SPEC-146 SecurityFields form (classification / share_mode / flags).
 * SSOT for dropzone, PDF admit, and detail Edit labels. Always labeled.
 */

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { Switch } from '@/components/ui/switch';
import { PrincipalSelect, principalDisplayName } from '@/components/security/principal-select';
import { useUsers } from '@/hooks/use-users';
import {
  CLASSIFICATIONS,
  DEFAULT_SECURITY_FIELDS,
  SHARE_MODES,
  classificationLabel,
  shareModeLabel,
  type Classification,
  type SecurityFields,
  type ShareMode,
} from '@/lib/security/security-fields';
import { ChevronDown, ChevronRight, Shield } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface SecurityFieldsFormProps {
  value: SecurityFields;
  onChange: (value: SecurityFields) => void;
  /** Compact layout for dropzone chrome — labels still render. */
  compact?: boolean;
  /** Start expanded (default: collapsed per UX spec). */
  defaultExpanded?: boolean;
  /** Hide the progressive-disclosure toggle (detail Edit labels). */
  alwaysExpanded?: boolean;
  /** Notify parent when body open state changes (documents-chrome height). */
  onExpandedChange?: (open: boolean) => void;
  className?: string;
}

export function SecurityFieldsForm({
  value,
  onChange,
  compact = false,
  defaultExpanded = false,
  alwaysExpanded = false,
  onExpandedChange,
  className,
}: SecurityFieldsFormProps) {
  const { t } = useTranslation();
  const fields = value ?? DEFAULT_SECURITY_FIELDS;
  const [expanded, setExpanded] = useState(defaultExpanded || alwaysExpanded);
  const { users, isLoading: usersLoading } = useUsers();

  const patch = (partial: Partial<SecurityFields>) => {
    onChange({ ...fields, ...partial });
  };

  const aclIds = fields.acl_principal_ids ?? [];
  const showBody = alwaysExpanded || expanded;
  const onExpandedChangeRef = useRef(onExpandedChange);
  onExpandedChangeRef.current = onExpandedChange;

  useEffect(() => {
    onExpandedChangeRef.current?.(showBody);
  }, [showBody]);

  const triggerClass = compact ? 'h-8 text-xs' : 'h-9';
  const summary = [
    t(
      `security.classification.${(fields.classification as string) || 'internal'}`,
      classificationLabel(fields.classification),
    ),
    t(`security.shareMode.${fields.share_mode}`, shareModeLabel(fields.share_mode)),
    fields.export_control ? t('security.exportControlShort', 'Export') : null,
    fields.pii ? t('security.piiShort', 'PII') : null,
    fields.project_id ? fields.project_id : null,
  ]
    .filter(Boolean)
    .join(' · ');

  const classificationSelect = (
    <div className="space-y-1 min-w-0">
      <Label className="text-xs" htmlFor="spec146-classification">
        {t('security.classification.label', 'Classification')}
      </Label>
      <Select
        value={(fields.classification as Classification) || 'internal'}
        onValueChange={(classification: Classification) => patch({ classification })}
      >
        <SelectTrigger
          id="spec146-classification"
          className={triggerClass}
          data-testid="spec146-classification"
        >
          <SelectValue
            placeholder={t('security.classification.label', 'Classification')}
          />
        </SelectTrigger>
        <SelectContent>
          {CLASSIFICATIONS.map((c) => (
            <SelectItem key={c.value} value={c.value}>
              {t(`security.classification.${c.value}`, c.label)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );

  const shareSelect = (
    <div className="space-y-1 min-w-0">
      <Label className="text-xs" htmlFor="spec146-share-mode">
        {t('security.shareMode.label', 'Share mode')}
      </Label>
      <Select
        value={fields.share_mode}
        onValueChange={(share_mode: ShareMode) => patch({ share_mode })}
      >
        <SelectTrigger
          id="spec146-share-mode"
          className={triggerClass}
          data-testid="spec146-share-mode"
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {SHARE_MODES.map((m) => (
            <SelectItem key={m.value} value={m.value}>
              {t(`security.shareMode.${m.value}`, m.label)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );

  const aclPicker = fields.share_mode === 'acl' && (
    <div className="space-y-1 min-w-0 sm:col-span-2">
      <Label className="text-xs">{t('security.aclPrincipals', 'ACL principals')}</Label>
      {compact ? (
        <Sheet>
          <SheetTrigger asChild>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 text-xs"
              data-testid="spec146-acl-picker"
            >
              {aclIds.length
                ? t('security.aclCount', '{{count}} principals', { count: aclIds.length })
                : t('security.aclAdd', 'Add principal…')}
            </Button>
          </SheetTrigger>
          <SheetContent side="bottom" className="max-h-[70vh]">
            <SheetHeader>
              <SheetTitle>{t('security.aclPrincipals', 'ACL principals')}</SheetTitle>
            </SheetHeader>
            <div className="mt-3 space-y-2">
              <PrincipalSelect
                users={users}
                allowEmpty
                isLoading={usersLoading}
                disabledIds={aclIds}
                testId="spec146-acl-select"
                placeholder={t('security.aclAdd', 'Add principal…')}
                onValueChange={(userId) => {
                  if (!userId || aclIds.includes(userId)) return;
                  patch({ acl_principal_ids: [...aclIds, userId] });
                }}
              />
              <AclChips
                aclIds={aclIds}
                users={users}
                onRemove={(id) =>
                  patch({ acl_principal_ids: aclIds.filter((x) => x !== id) })
                }
              />
            </div>
          </SheetContent>
        </Sheet>
      ) : (
        <>
          <PrincipalSelect
            users={users}
            allowEmpty
            isLoading={usersLoading}
            disabledIds={aclIds}
            testId="spec146-acl-picker"
            placeholder={t('security.aclAdd', 'Add principal…')}
            onValueChange={(userId) => {
              if (!userId || aclIds.includes(userId)) return;
              patch({ acl_principal_ids: [...aclIds, userId] });
            }}
          />
          <AclChips
            aclIds={aclIds}
            users={users}
            onRemove={(id) =>
              patch({ acl_principal_ids: aclIds.filter((x) => x !== id) })
            }
          />
        </>
      )}
    </div>
  );

  return (
    <div
      className={className}
      data-testid="spec146-security-fields"
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
    >
      {!alwaysExpanded && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs text-muted-foreground gap-1"
          data-testid="spec146-security-toggle"
          aria-expanded={showBody}
          onClick={(e) => {
            e.stopPropagation();
            setExpanded((v) => !v);
          }}
        >
          <Shield className="h-3.5 w-3.5" aria-hidden="true" />
          {t('security.fields.title', 'Security')}
          {showBody ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
          {!showBody && (
            <span
              className="ml-1 font-normal truncate max-w-[14rem]"
              data-testid="spec146-security-summary"
            >
              {summary}
            </span>
          )}
        </Button>
      )}

      {showBody && (
        <div className="mt-2 grid gap-3 sm:grid-cols-2">
          {classificationSelect}
          {shareSelect}
          {aclPicker}

          <div className="flex items-center gap-2">
            <Switch
              id="spec146-export-control"
              checked={fields.export_control}
              onCheckedChange={(export_control) => patch({ export_control })}
              data-testid="spec146-export-control"
            />
            <Label
              htmlFor="spec146-export-control"
              className="text-xs text-muted-foreground"
            >
              {t('security.exportControl', 'Export control')}
            </Label>
          </div>

          <div className="flex items-center gap-2">
            <Switch
              id="spec146-pii"
              checked={fields.pii}
              onCheckedChange={(pii) => patch({ pii })}
              data-testid="spec146-pii"
            />
            <Label htmlFor="spec146-pii" className="text-xs text-muted-foreground">
              {t('security.pii', 'PII')}
            </Label>
          </div>

          <div className="space-y-1 min-w-0 sm:col-span-2">
            <Label className="text-xs" htmlFor="spec146-project-id">
              {t('security.projectId', 'Project ID (optional)')}
            </Label>
            <Input
              id="spec146-project-id"
              className={triggerClass}
              value={fields.project_id ?? ''}
              placeholder={t(
                'security.projectIdPlaceholder',
                'Free-text project id — no catalog',
              )}
              onChange={(e) =>
                patch({ project_id: e.target.value || undefined })
              }
              data-testid="spec146-project-id"
            />
          </div>
        </div>
      )}
    </div>
  );
}

function AclChips({
  aclIds,
  users,
  onRemove,
}: {
  aclIds: string[];
  users: { user_id: string; username: string; email: string }[];
  onRemove: (id: string) => void;
}) {
  if (aclIds.length === 0) return null;
  return (
    <ul className="flex flex-wrap gap-1 mt-1" data-testid="spec146-acl-list">
      {aclIds.map((id) => {
        const u = users.find((x) => x.user_id === id);
        return (
          <li key={id}>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="h-6 text-xs"
              onClick={() => onRemove(id)}
            >
              {principalDisplayName(u, id)} ×
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
