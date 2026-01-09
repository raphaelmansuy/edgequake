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
import { Slider } from '@/components/ui/slider';
import { Textarea } from '@/components/ui/textarea';
import { updateRelationship } from '@/lib/api/edgequake';
import type { GraphEdge, Relationship } from '@/types';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import {
    ArrowRight,
    Edit,
    Loader2,
    Scale,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

// Common relationship types
const RELATIONSHIP_TYPES = [
  'RELATED_TO',
  'PART_OF',
  'BELONGS_TO',
  'CONTAINS',
  'WORKS_FOR',
  'WORKS_WITH',
  'LOCATED_IN',
  'CREATED_BY',
  'MENTIONED_IN',
  'REFERENCES',
  'DEPENDS_ON',
  'PRECEDES',
  'FOLLOWS',
  'CAUSES',
  'AFFECTS',
  'OTHER',
];

interface RelationshipEditDialogProps {
  /**
   * The edge/relationship to edit. When null, dialog is closed.
   */
  edge: GraphEdge | Relationship | null;
  /**
   * Source node label for display
   */
  sourceLabel?: string;
  /**
   * Target node label for display
   */
  targetLabel?: string;
  /**
   * Whether the dialog is open
   */
  open: boolean;
  /**
   * Callback when dialog open state changes
   */
  onOpenChange: (open: boolean) => void;
  /**
   * Callback when relationship is successfully updated
   */
  onUpdated?: (relationship: Relationship) => void;
}

export function RelationshipEditDialog({
  edge,
  sourceLabel,
  targetLabel,
  open,
  onOpenChange,
  onUpdated,
}: RelationshipEditDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // Form state
  const [relationshipType, setRelationshipType] = useState('');
  const [description, setDescription] = useState('');
  const [weight, setWeight] = useState(1);
  const [customType, setCustomType] = useState('');

  // Original values for comparison
  const [originalType, setOriginalType] = useState('');
  const [originalDescription, setOriginalDescription] = useState('');
  const [originalWeight, setOriginalWeight] = useState(1);

  // Initialize form when edge changes
  useEffect(() => {
    if (edge) {
      const edgeType = edge.relationship_type || 'RELATED_TO';
      const edgeDescription = edge.description || '';
      const edgeWeight = edge.weight ?? 1;

      // Intentional: Form initialization from props
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setRelationshipType(edgeType);
      setOriginalType(edgeType);
      setDescription(edgeDescription);
      setOriginalDescription(edgeDescription);
      setWeight(edgeWeight);
      setOriginalWeight(edgeWeight);

      // Check if it's a custom type
      if (!RELATIONSHIP_TYPES.includes(edgeType)) {
        setCustomType(edgeType);
        setRelationshipType('OTHER');
      } else {
        setCustomType('');
      }
    }
  }, [edge]);

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: (data: {
      relationship_type?: string;
      description?: string;
      weight?: number;
    }) => updateRelationship(edge!.id, data),
    onSuccess: (updatedRelationship) => {
      toast.success(
        t('relationship.updateSuccess', 'Relationship updated successfully')
      );
      queryClient.invalidateQueries({ queryKey: ['graph'] });
      queryClient.invalidateQueries({ queryKey: ['relationships'] });
      onUpdated?.(updatedRelationship);
      onOpenChange(false);
    },
    onError: (error) => {
      toast.error(
        t('relationship.updateFailed', 'Failed to update relationship'),
        {
          description: error instanceof Error ? error.message : 'Unknown error',
        }
      );
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const actualType =
      relationshipType === 'OTHER' ? customType.toUpperCase() : relationshipType;
    const updates: Partial<Relationship> = {};

    if (actualType !== originalType) {
      updates.relationship_type = actualType;
    }
    if (description !== originalDescription) {
      updates.description = description;
    }
    if (weight !== originalWeight) {
      updates.weight = weight;
    }

    if (Object.keys(updates).length === 0) {
      toast.info(t('relationship.noChanges', 'No changes to save'));
      return;
    }

    updateMutation.mutate(updates);
  };

  const actualType =
    relationshipType === 'OTHER' ? customType.toUpperCase() : relationshipType;
  const hasChanges =
    actualType !== originalType ||
    description !== originalDescription ||
    weight !== originalWeight;

  if (!edge) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Edit className="h-5 w-5" />
            {t('relationship.edit', 'Edit Relationship')}
          </DialogTitle>
          <DialogDescription>
            {t(
              'relationship.editDescription',
              'Modify the relationship properties between entities.'
            )}
          </DialogDescription>
        </DialogHeader>

        {/* Visual representation of the relationship */}
        <div className="flex items-center justify-center gap-2 py-4 px-2 bg-muted/50 rounded-lg">
          <div className="text-center">
            <Badge variant="secondary" className="max-w-[100px] truncate">
              {sourceLabel ||
                ('source' in edge ? edge.source : edge.source_entity_id)}
            </Badge>
          </div>
          <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
          <div className="text-center px-2 py-1 bg-primary/10 rounded">
            <Badge className="text-xs">{actualType || '...'}</Badge>
          </div>
          <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
          <div className="text-center">
            <Badge variant="secondary" className="max-w-[100px] truncate">
              {targetLabel ||
                ('target' in edge ? edge.target : edge.target_entity_id)}
            </Badge>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Relationship Type */}
          <div className="space-y-2">
            <Label htmlFor="relationship-type">
              {t('relationship.type', 'Relationship Type')}
            </Label>
            <Select value={relationshipType} onValueChange={setRelationshipType}>
              <SelectTrigger id="relationship-type">
                <SelectValue
                  placeholder={t('relationship.selectType', 'Select type...')}
                />
              </SelectTrigger>
              <SelectContent>
                {RELATIONSHIP_TYPES.map((type) => (
                  <SelectItem key={type} value={type}>
                    {type.replace(/_/g, ' ')}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Custom Type (when OTHER is selected) */}
          {relationshipType === 'OTHER' && (
            <div className="space-y-2">
              <Label htmlFor="custom-type">
                {t('relationship.customType', 'Custom Type')}
              </Label>
              <Input
                id="custom-type"
                value={customType}
                onChange={(e) => setCustomType(e.target.value.toUpperCase())}
                placeholder={t('relationship.customTypePlaceholder', 'CUSTOM_TYPE')}
                className="uppercase"
              />
            </div>
          )}

          {/* Weight */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <Label className="flex items-center gap-1">
                <Scale className="h-3 w-3" />
                {t('relationship.weight', 'Weight')}
              </Label>
              <span className="text-sm font-medium">{weight.toFixed(2)}</span>
            </div>
            <Slider
              value={[weight]}
              onValueChange={([val]) => setWeight(val)}
              min={0}
              max={1}
              step={0.05}
              className="w-full"
            />
            <p className="text-xs text-muted-foreground">
              {t(
                'relationship.weightHint',
                'Higher weight indicates stronger relationship (0.0 - 1.0)'
              )}
            </p>
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="relationship-description">
              {t('relationship.description', 'Description')}
            </Label>
            <Textarea
              id="relationship-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t(
                'relationship.descriptionPlaceholder',
                'Describe this relationship...'
              )}
              rows={3}
            />
          </div>

          {/* Source IDs (read-only) */}
          {edge.source_ids && edge.source_ids.length > 0 && (
            <div className="space-y-2">
              <Label className="text-xs text-muted-foreground">
                {t('relationship.sourceDocuments', 'Source Documents')}
              </Label>
              <div className="flex flex-wrap gap-1">
                {edge.source_ids.slice(0, 5).map((id, idx) => (
                  <Badge key={idx} variant="outline" className="text-[10px]">
                    {id.slice(0, 8)}...
                  </Badge>
                ))}
                {edge.source_ids.length > 5 && (
                  <Badge variant="outline" className="text-[10px]">
                    +{edge.source_ids.length - 5} more
                  </Badge>
                )}
              </div>
            </div>
          )}

          <DialogFooter className="gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={updateMutation.isPending}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              type="submit"
              disabled={!hasChanges || updateMutation.isPending}
            >
              {updateMutation.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('common.save', 'Save Changes')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export default RelationshipEditDialog;
