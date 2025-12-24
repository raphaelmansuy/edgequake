'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogDescription,
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
import { Textarea } from '@/components/ui/textarea';
import { mergeEntities, updateEntity } from '@/lib/api/edgequake';
import type { Entity, GraphNode } from '@/types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
    AlertTriangle,
    Edit,
    GitMerge,
    Loader2,
    Sparkles,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

// Common entity types
const ENTITY_TYPES = [
  'PERSON',
  'ORGANIZATION',
  'LOCATION',
  'EVENT',
  'CONCEPT',
  'DOCUMENT',
  'PRODUCT',
  'TECHNOLOGY',
  'DATE',
  'OTHER',
];

interface EntityEditDialogProps {
  /**
   * The node/entity to edit. When null, dialog is closed.
   */
  node: GraphNode | Entity | null;
  /**
   * Whether the dialog is open
   */
  open: boolean;
  /**
   * Callback when dialog open state changes
   */
  onOpenChange: (open: boolean) => void;
  /**
   * Callback when entity is successfully updated
   */
  onUpdated?: (entity: Entity) => void;
  /**
   * Optional list of other entities for merge target selection
   */
  otherEntities?: Array<{ id: string; label: string; entity_type: string }>;
}

interface MergeConflictState {
  show: boolean;
  existingEntity?: { id: string; label: string; entity_type: string };
  newLabel: string;
}

export function EntityEditDialog({
  node,
  open,
  onOpenChange,
  onUpdated,
  otherEntities = [],
}: EntityEditDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Form state
  const [label, setLabel] = useState('');
  const [description, setDescription] = useState('');
  const [entityType, setEntityType] = useState('');
  const [originalLabel, setOriginalLabel] = useState('');

  // Merge conflict state
  const [mergeConflict, setMergeConflict] = useState<MergeConflictState>({
    show: false,
    newLabel: '',
  });

  // Initialize form when node changes
  useEffect(() => {
    if (node) {
      const nodeLabel = node.label || '';
      const nodeType = 'entity_type' in node ? node.entity_type : node.node_type;
      
      setLabel(nodeLabel);
      setOriginalLabel(nodeLabel);
      setDescription(node.description || '');
      setEntityType(nodeType || '');
    }
  }, [node]);

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: (data: {
      label?: string;
      description?: string;
      entity_type?: string;
    }) => updateEntity(node!.id, data),
    onSuccess: (updatedEntity) => {
      toast.success(t('entity.updateSuccess', 'Entity updated successfully'));
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      queryClient.invalidateQueries({ queryKey: ['entities'] });
      onUpdated?.(updatedEntity);
      onOpenChange(false);
    },
    onError: (error) => {
      // Check if it's a rename conflict (409 Conflict)
      if (error instanceof Error && error.message.includes('409')) {
        // Show merge conflict dialog
        const conflictingEntity = otherEntities.find(
          (e) => e.label.toLowerCase() === label.toLowerCase() && e.id !== node?.id
        );
        
        if (conflictingEntity) {
          setMergeConflict({
            show: true,
            existingEntity: conflictingEntity,
            newLabel: label,
          });
        } else {
          toast.error(
            t('entity.updateFailed', 'Failed to update entity'),
            {
              description: t(
                'entity.labelConflict',
                'An entity with this name already exists. Consider merging instead.'
              ),
            }
          );
        }
      } else {
        toast.error(
          t('entity.updateFailed', 'Failed to update entity'),
          {
            description: error instanceof Error ? error.message : 'Unknown error',
          }
        );
      }
    },
  });

  // Merge mutation
  const mergeMutation = useMutation({
    mutationFn: (targetEntityId: string) =>
      mergeEntities({
        source_ids: [node!.id],
        target_label: mergeConflict.existingEntity?.label || label,
        target_type: mergeConflict.existingEntity?.entity_type || entityType,
      }),
    onSuccess: (result) => {
      toast.success(
        t('entity.mergeSuccess', 'Entities merged successfully'),
        {
          description: t(
            'entity.mergeSuccessDesc',
            'Merged {{count}} entities into one.',
            { count: result.merged_count }
          ),
        }
      );
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      queryClient.invalidateQueries({ queryKey: ['entities'] });
      setMergeConflict({ show: false, newLabel: '' });
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error(
        t('entity.mergeFailed', 'Failed to merge entities'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const updates: Record<string, string> = {};

    if (label !== originalLabel) {
      updates.label = label;
    }
    if (description !== (node?.description || '')) {
      updates.description = description;
    }
    const nodeType = 'entity_type' in node! ? node!.entity_type : node!.node_type;
    if (entityType !== nodeType) {
      updates.entity_type = entityType;
    }

    if (Object.keys(updates).length === 0) {
      toast.info(t('entity.noChanges', 'No changes to save'));
      return;
    }

    updateMutation.mutate(updates);
  };

  const handleMerge = () => {
    if (mergeConflict.existingEntity) {
      mergeMutation.mutate(mergeConflict.existingEntity.id);
    }
  };

  const handleCancelMerge = () => {
    setMergeConflict({ show: false, newLabel: '' });
    // Revert to original label
    setLabel(originalLabel);
  };

  const isLoading = updateMutation.isPending || mergeMutation.isPending;
  const hasChanges =
    label !== originalLabel ||
    description !== (node?.description || '') ||
    entityType !== ('entity_type' in (node || {}) ? (node as Entity).entity_type : (node as GraphNode)?.node_type);

  if (!node) return null;

  return (
    <>
      <Dialog open={open && !mergeConflict.show} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Edit className="h-5 w-5" />
              {t('entity.edit', 'Edit Entity')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'entity.editDescription',
                'Modify the entity properties. Renaming may trigger a merge if another entity with the same name exists.'
              )}
            </DialogDescription>
          </DialogHeader>

          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Label */}
            <div className="space-y-2">
              <Label htmlFor="entity-label">
                {t('entity.label', 'Entity Name')}
              </Label>
              <Input
                id="entity-label"
                value={label}
                onChange={(e) => setLabel(e.target.value)}
                placeholder={t('entity.labelPlaceholder', 'Entity name')}
                className="font-medium"
              />
              {label !== originalLabel && (
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <AlertTriangle className="h-3 w-3 text-yellow-500" />
                  {t(
                    'entity.renameWarning',
                    'Renaming may trigger a merge if a duplicate exists'
                  )}
                </p>
              )}
            </div>

            {/* Entity Type */}
            <div className="space-y-2">
              <Label htmlFor="entity-type">
                {t('entity.type', 'Entity Type')}
              </Label>
              <Select value={entityType} onValueChange={setEntityType}>
                <SelectTrigger id="entity-type">
                  <SelectValue
                    placeholder={t('entity.selectType', 'Select type...')}
                  />
                </SelectTrigger>
                <SelectContent>
                  {ENTITY_TYPES.map((type) => (
                    <SelectItem key={type} value={type}>
                      {type}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Description */}
            <div className="space-y-2">
              <Label htmlFor="entity-description">
                {t('entity.description', 'Description')}
              </Label>
              <Textarea
                id="entity-description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder={t(
                  'entity.descriptionPlaceholder',
                  'A brief description of this entity...'
                )}
                rows={3}
              />
            </div>

            {/* Properties Display (read-only) */}
            {'properties' in node && node.properties && Object.keys(node.properties).length > 0 && (
              <div className="space-y-2">
                <Label className="flex items-center gap-1">
                  <Sparkles className="h-3 w-3" />
                  {t('entity.properties', 'Properties')}
                </Label>
                <div className="bg-muted/50 rounded-md p-2 text-xs space-y-1">
                  {Object.entries(node.properties).map(([key, value]) => (
                    <div key={key} className="flex justify-between">
                      <span className="text-muted-foreground">{key}</span>
                      <span className="font-medium">{String(value)}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <DialogFooter className="gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={isLoading}
              >
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button type="submit" disabled={!hasChanges || isLoading}>
                {isLoading && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                {t('common.save', 'Save Changes')}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {/* Merge Conflict Dialog */}
      <Dialog
        open={mergeConflict.show}
        onOpenChange={(open) => !open && handleCancelMerge()}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-yellow-600">
              <AlertTriangle className="h-5 w-5" />
              {t('entity.mergeConflict', 'Merge Conflict')}
            </DialogTitle>
            <DialogDescription>
              {t(
                'entity.mergeConflictDescription',
                'An entity with this name already exists. Would you like to merge them?'
              )}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="flex items-center justify-between gap-4 p-3 bg-muted/50 rounded-lg">
              <div className="space-y-1">
                <p className="text-sm font-medium">
                  {t('entity.currentEntity', 'Current Entity')}
                </p>
                <Badge variant="outline">{originalLabel}</Badge>
              </div>
              <GitMerge className="h-5 w-5 text-muted-foreground" />
              <div className="space-y-1 text-right">
                <p className="text-sm font-medium">
                  {t('entity.existingEntity', 'Existing Entity')}
                </p>
                <Badge variant="outline">
                  {mergeConflict.existingEntity?.label}
                </Badge>
              </div>
            </div>

            <p className="text-sm text-muted-foreground">
              {t(
                'entity.mergeExplanation',
                'Merging will combine all relationships and properties from both entities into one. This action cannot be undone.'
              )}
            </p>
          </div>

          <DialogFooter className="gap-2">
            <Button
              variant="outline"
              onClick={handleCancelMerge}
              disabled={mergeMutation.isPending}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              variant="default"
              onClick={handleMerge}
              disabled={mergeMutation.isPending}
            >
              {mergeMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              <GitMerge className="mr-2 h-4 w-4" />
              {t('entity.merge', 'Merge Entities')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

export default EntityEditDialog;
