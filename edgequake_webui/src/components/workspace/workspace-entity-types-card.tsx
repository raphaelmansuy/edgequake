/**
 * @module WorkspaceEntityTypesCard
 * @description Entity type list + strict-limit editor for workspace settings (dashboard and deeplink).
 *
 * @implements SPEC-085 / GitHub #216 — editable entity_types
 * @implements SPEC-013 entity_extraction — entity_types_strict toggle
 */
'use client';

import { EntityTypeSelector } from '@/components/shared/entity-type-selector';
import { Badge } from '@/components/ui/badge';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { Workspace } from '@/types';
import { Tags } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WorkspaceEntityTypesCardProps {
  isEditing: boolean;
  workspace: Workspace;
  selectedTypes: string[];
  onTypesChange: (types: string[]) => void;
  strictLimit: boolean;
  onStrictLimitChange: (strict: boolean) => void;
}

export function WorkspaceEntityTypesCard({
  isEditing,
  workspace,
  selectedTypes,
  onTypesChange,
  strictLimit,
  onStrictLimitChange,
}: WorkspaceEntityTypesCardProps) {
  const { t } = useTranslation();

  return (
    <Card data-testid="workspace-entity-types-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Tags className="h-5 w-5 text-indigo-600" />
          {t('entityTypes.title', 'Entity Types')}
        </CardTitle>
        <CardDescription>
          {t(
            'entityTypes.futureOnlyHint',
            'Applies to future document ingestions. Use Rebuild Knowledge Graph to re-extract existing documents.'
          )}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {isEditing ? (
          <EntityTypeSelector
            value={selectedTypes}
            onChange={onTypesChange}
            strictLimit={strictLimit}
            onStrictLimitChange={onStrictLimitChange}
          />
        ) : workspace.entity_types && workspace.entity_types.length > 0 ? (
          <div className="space-y-3">
            <div className="flex flex-wrap gap-1.5">
              {workspace.entity_types.map((type) => (
                <Badge
                  key={type}
                  variant="secondary"
                  className="text-xs font-mono"
                  data-testid={`ws-entity-type-${type}`}
                >
                  {type}
                </Badge>
              ))}
            </div>
            <p className="text-xs text-muted-foreground" data-testid="entity-types-strict-status">
              {workspace.entity_types_strict !== false
                ? t('entityTypes.strictOn', 'Strict limit: on (unknown types → OTHER)')
                : t('entityTypes.strictOff', 'Strict limit: off (free-form types allowed)')}
            </p>
          </div>
        ) : (
          <div className="text-sm text-muted-foreground space-y-1">
            <span className="font-medium">
              {t('entityTypes.defaults', 'Using server defaults:')}
            </span>{' '}
            <span className="font-mono text-xs">
              {t(
                'entityTypes.defaultsHint',
                'PERSON, ORGANIZATION, LOCATION, EVENT, CONCEPT, TECHNOLOGY, PRODUCT, DATE, DOCUMENT'
              )}
            </span>
            <p className="text-xs" data-testid="entity-types-strict-status">
              {workspace.entity_types_strict !== false
                ? t('entityTypes.strictOn', 'Strict limit: on (unknown types → OTHER)')
                : t('entityTypes.strictOff', 'Strict limit: off (free-form types allowed)')}
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
