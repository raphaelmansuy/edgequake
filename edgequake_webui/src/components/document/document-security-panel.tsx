'use client';

/**
 * SPEC-146 document detail Security panel — same SecurityFieldsForm as upload.
 */

import { Button } from '@/components/ui/button';
import { ClassificationChip } from '@/components/security/classification-chip';
import { ShareModeBadge } from '@/components/security/share-mode-badge';
import { QuarantineBanner } from '@/components/security/quarantine-banner';
import { SecurityFieldsForm } from '@/components/security/security-fields-form';
import { useCanSetLabels } from '@/hooks/use-can-set-labels';
import { useDocAbacEnabled } from '@/hooks/use-doc-abac';
import { patchDocumentSecurityLabels } from '@/lib/api/edgequake/documents';
import {
  DEFAULT_SECURITY_FIELDS,
  type SecurityFields,
  type ShareMode,
} from '@/lib/security/security-fields';
import type { Document } from '@/types';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

function fieldsFromDocument(doc: Document): SecurityFields {
  return {
    ...DEFAULT_SECURITY_FIELDS,
    classification: doc.classification || 'internal',
    share_mode: (doc.share_mode as ShareMode) || 'workspace',
    security_status: doc.security_status === 'quarantined' ? 'quarantined' : 'ok',
    export_control: Boolean(doc.export_control),
    pii: Boolean(doc.pii),
    project_id: doc.project_id || undefined,
  };
}

export function DocumentSecurityPanel({
  document: doc,
  onSaved,
}: {
  document: Document;
  onSaved?: () => void;
}) {
  const { t } = useTranslation();
  const { docAbacEnabled } = useDocAbacEnabled();
  const canSetLabels = useCanSetLabels();
  const formRef = useRef<HTMLDivElement>(null);
  const [fields, setFields] = useState<SecurityFields>(() => fieldsFromDocument(doc));
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setFields(fieldsFromDocument(doc));
  }, [doc]);

  if (!docAbacEnabled) return null;

  const quarantined = doc.security_status === 'quarantined';

  const save = async () => {
    if (!canSetLabels) return;
    setSaving(true);
    try {
      await patchDocumentSecurityLabels(doc.id, fields);
      toast.success(t('security.saveLabels', 'Save labels'));
      onSaved?.();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Save labels failed');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-3" data-testid="spec146-security-panel">
      <div className="flex flex-wrap items-center gap-1.5">
        <ClassificationChip value={doc.classification} />
        <ShareModeBadge value={doc.share_mode} />
      </div>
      {quarantined ? (
        <QuarantineBanner
          canSetLabels={canSetLabels}
          onRetry={() => {
            formRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
            formRef.current
              ?.querySelector<HTMLElement>('[data-testid="spec146-classification"]')
              ?.focus();
          }}
        />
      ) : null}
      {canSetLabels ? (
        <>
          <div ref={formRef}>
            <SecurityFieldsForm value={fields} onChange={setFields} alwaysExpanded />
          </div>
          <Button
            size="sm"
            onClick={() => void save()}
            disabled={saving}
            data-testid="spec146-save-labels"
          >
            {saving
              ? t('common.saving', 'Saving…')
              : t('security.saveLabels', 'Save labels')}
          </Button>
        </>
      ) : (
        <p className="text-xs text-muted-foreground" data-testid="spec146-no-set-labels">
          {t(
            'security.noSetLabels',
            'You don’t have permission to change security labels.',
          )}
        </p>
      )}
    </div>
  );
}
