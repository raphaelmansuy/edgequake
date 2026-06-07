'use client';

/**
 * @module EntityTypeSelector
 * @description Reusable entity type picker for workspace creation.
 *
 * Provides:
 * - Preset buttons (General, Manufacturing, Healthcare, Legal, Research, Finance)
 * - Chip list of active types with individual remove buttons
 * - Text input to add custom UPPERCASE_UNDERSCORED types
 * - Max-50 enforcement with visual feedback
 *
 * @implements SPEC-085: Custom entity configuration from UI
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
    ENTITY_PRESETS,
    MAX_ENTITY_TYPES,
    type PresetKey,
    deduplicateTypes,
    detectPreset,
    normalizeEntityType,
} from '@/constants/entity-presets';
import {
    Factory,
    FlaskConical,
    Globe,
    HeartPulse,
    Plus,
    Scale,
    TrendingUp,
    X,
} from 'lucide-react';
import { KeyboardEvent, useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

// ─── Icon helper ────────────────────────────────────────────────────────────

const PRESET_ICONS: Record<string, React.ReactNode> = {
  Globe: <Globe className="h-3.5 w-3.5" />,
  Factory: <Factory className="h-3.5 w-3.5" />,
  HeartPulse: <HeartPulse className="h-3.5 w-3.5" />,
  Scale: <Scale className="h-3.5 w-3.5" />,
  FlaskConical: <FlaskConical className="h-3.5 w-3.5" />,
  TrendingUp: <TrendingUp className="h-3.5 w-3.5" />,
};

// ─── Props ───────────────────────────────────────────────────────────────────

export interface EntityTypeSelectorProps {
  /** Current entity types (controlled). */
  value: string[];
  /** Called whenever the list changes. */
  onChange: (types: string[]) => void;
  /** When true, disable all interactions (read-only display). */
  readOnly?: boolean;
  /** Strict mode: limit to listed types and remap unknowns to OTHER. */
  strictLimit?: boolean;
  /** Called when strict-limit checkbox changes (omit to hide checkbox). */
  onStrictLimitChange?: (strict: boolean) => void;
}

// ─── Component ───────────────────────────────────────────────────────────────

/**
 * Entity type selector with preset buttons and chip list.
 *
 * Usage:
 * ```tsx
 * const [entityTypes, setEntityTypes] = useState(ENTITY_PRESETS.general.types);
 * <EntityTypeSelector value={entityTypes} onChange={setEntityTypes} />
 * ```
 *
 * @implements SPEC-085: Entity type UI from ADR-0005 Step 6
 */
export function EntityTypeSelector({
  value,
  onChange,
  readOnly = false,
  strictLimit = true,
  onStrictLimitChange,
}: EntityTypeSelectorProps) {
  const { t } = useTranslation();
  const [customInput, setCustomInput] = useState('');
  const [advancedBulkInput, setAdvancedBulkInput] = useState('');

  const activePreset: PresetKey = useMemo(() => detectPreset(value), [value]);
  const atMax = value.length >= MAX_ENTITY_TYPES;

  // ── Preset selection ────────────────────────────────────────────────────
  const handlePresetClick = useCallback(
    (key: PresetKey) => {
      if (readOnly || key === 'custom') return;
      onChange([...ENTITY_PRESETS[key].types]);
    },
    [onChange, readOnly]
  );

  // ── Remove individual type ───────────────────────────────────────────────
  const handleRemove = useCallback(
    (type: string) => {
      if (readOnly) return;
      onChange(value.filter((t) => t !== type));
    },
    [onChange, readOnly, value]
  );

  // ── Add custom type ──────────────────────────────────────────────────────
  const handleAdd = useCallback(() => {
    if (readOnly) return;
    const normalized = normalizeEntityType(customInput);
    if (!normalized) return;
    const next = deduplicateTypes([...value, normalized]);
    onChange(next);
    setCustomInput('');
  }, [customInput, onChange, readOnly, value]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd]
  );

  // ── Advanced actions ──────────────────────────────────────────────────────
  const handleApplyBulk = useCallback(() => {
    if (readOnly) return;
    const parsed = advancedBulkInput
      .split(/[\n,;]+/)
      .map((entry) => normalizeEntityType(entry))
      .filter(Boolean);
    onChange(deduplicateTypes(parsed));
  }, [advancedBulkInput, onChange, readOnly]);

  const handleResetGeneral = useCallback(() => {
    if (readOnly) return;
    onChange([...ENTITY_PRESETS.general.types]);
  }, [onChange, readOnly]);

  const handleSortAZ = useCallback(() => {
    if (readOnly) return;
    onChange([...value].sort((a, b) => a.localeCompare(b)));
  }, [onChange, readOnly, value]);

  const handleClearAll = useCallback(() => {
    if (readOnly) return;
    onChange([]);
  }, [onChange, readOnly]);

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div
      className="space-y-3"
      data-testid="entity-type-selector"
      aria-label={t('entityTypes.selectorLabel', 'Entity type selector')}
    >
      <Tabs defaultValue="basic" className="space-y-3">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="basic" data-testid="entity-tab-basic">
            {t('entityTypes.tabBasic', 'Basic')}
          </TabsTrigger>
          <TabsTrigger value="advanced" data-testid="entity-tab-advanced">
            {t('entityTypes.tabAdvanced', 'Advanced')}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="basic" className="space-y-3 m-0">
          {/* Preset buttons */}
          <div className="space-y-1.5">
            <p className="text-xs font-medium text-muted-foreground">
              {t('entityTypes.presetLabel', 'Domain Preset')}
            </p>
            <div className="flex flex-wrap gap-1.5">
              {(Object.entries(ENTITY_PRESETS) as [Exclude<PresetKey, 'custom'>, typeof ENTITY_PRESETS[keyof typeof ENTITY_PRESETS]][]).map(
                ([key, preset]) => {
                  const isActive = activePreset === key;
                  return (
                    <Button
                      key={key}
                      type="button"
                      size="sm"
                      variant={isActive ? 'default' : 'outline'}
                      className="h-7 gap-1.5 px-2.5 text-xs"
                      onClick={() => handlePresetClick(key)}
                      disabled={readOnly}
                      data-testid={`preset-btn-${key}`}
                      aria-pressed={isActive}
                    >
                      {PRESET_ICONS[preset.icon]}
                      {t(preset.labelKey, preset.labelFallback)}
                    </Button>
                  );
                }
              )}
              {activePreset === 'custom' && (
                <Button
                  type="button"
                  size="sm"
                  variant="default"
                  className="h-7 gap-1.5 px-2.5 text-xs"
                  disabled
                  data-testid="preset-btn-custom"
                  aria-pressed
                >
                  {t('entityTypes.presets.custom', 'Custom')}
                </Button>
              )}
            </div>
          </div>

          {/* Active type chips */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <p className="text-xs font-medium text-muted-foreground">
                {t('entityTypes.typesLabel', 'Entity Types')}
              </p>
              <span
                className={`text-xs ${atMax ? 'text-destructive' : 'text-muted-foreground'}`}
                aria-live="polite"
              >
                {value.length}/{MAX_ENTITY_TYPES}
              </span>
            </div>

            <div
              className="min-h-10 flex flex-wrap gap-1.5 p-2 rounded-md border bg-background"
              data-testid="entity-types-chips"
            >
              {value.length === 0 && (
                <span className="text-xs text-muted-foreground italic self-center">
                  {t('entityTypes.emptyHint', 'No types selected — server defaults will be used')}
                </span>
              )}
              {value.map((type) => (
                <Badge
                  key={type}
                  variant="secondary"
                  className="gap-1 pr-1 text-xs font-mono"
                  data-testid={`entity-type-chip-${type}`}
                >
                  {type}
                  {!readOnly && (
                    <button
                      type="button"
                      className="ml-0.5 rounded-sm hover:bg-muted-foreground/20 p-0.5 transition-colors"
                      onClick={() => handleRemove(type)}
                      aria-label={t('entityTypes.removeType', 'Remove {{type}}', { type })}
                      data-testid={`remove-type-${type}`}
                    >
                      <X className="h-3 w-3" />
                    </button>
                  )}
                </Badge>
              ))}
            </div>
          </div>

          {onStrictLimitChange && (
            <div className="flex items-start gap-2 rounded-md border bg-muted/30 p-3">
              <Checkbox
                id="entity-types-strict-limit"
                checked={strictLimit}
                onCheckedChange={(checked) =>
                  onStrictLimitChange(checked === true)
                }
                disabled={readOnly}
                data-testid="entity-types-strict-checkbox"
              />
              <div className="space-y-0.5">
                <Label
                  htmlFor="entity-types-strict-limit"
                  className="text-xs font-medium leading-snug cursor-pointer"
                >
                  {t(
                    'entityTypes.strictLimitLabel',
                    'Limit extraction to listed types (classify others as OTHER)'
                  )}
                </Label>
                <p className="text-xs text-muted-foreground">
                  {t(
                    'entityTypes.strictLimitHint',
                    'When off, the model may use additional type labels; unknown types are not forced into OTHER.'
                  )}
                </p>
              </div>
            </div>
          )}

          {/* Add custom type */}
          {!readOnly && (
            <div className="space-y-1.5">
              <p className="text-xs font-medium text-muted-foreground">
                {t('entityTypes.addCustom', 'Add Custom Type')}
              </p>
              <div className="flex gap-2">
                <Input
                  placeholder={t('entityTypes.addPlaceholder', 'e.g. CIRCUIT_BOARD')}
                  value={customInput}
                  onChange={(e) => setCustomInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  disabled={atMax}
                  className="h-8 text-xs font-mono uppercase"
                  data-testid="entity-type-input"
                  aria-label={t('entityTypes.addCustom', 'Add Custom Type')}
                />
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="h-8 px-3 shrink-0"
                  onClick={handleAdd}
                  disabled={atMax || !customInput.trim()}
                  data-testid="entity-type-add-btn"
                >
                  <Plus className="h-3.5 w-3.5 mr-1" />
                  {t('common.add', 'Add')}
                </Button>
              </div>
              {atMax && (
                <p className="text-xs text-destructive" role="alert">
                  {t('entityTypes.maxReached', 'Maximum of {{max}} entity types reached.', {
                    max: MAX_ENTITY_TYPES,
                  })}
                </p>
              )}
            </div>
          )}
        </TabsContent>

        <TabsContent value="advanced" className="m-0">
          <div className="grid gap-3 lg:grid-cols-2">
            <div className="rounded-md border p-3 space-y-2">
              <p className="text-xs font-medium text-muted-foreground">
                {t('entityTypes.bulkReplace', 'Bulk Replace Entity Types')}
              </p>
              <textarea
                value={advancedBulkInput}
                onChange={(e) => setAdvancedBulkInput(e.target.value)}
                placeholder={t(
                  'entityTypes.bulkPlaceholder',
                  'PERSON, ORGANIZATION, MACHINE or one per line'
                )}
                disabled={readOnly}
                className="w-full min-h-20 rounded-md border bg-background px-3 py-2 text-xs font-mono"
                data-testid="entity-advanced-bulk-input"
              />
              <div className="grid gap-2">
                <Button
                  type="button"
                  size="sm"
                  onClick={handleApplyBulk}
                  disabled={readOnly || !advancedBulkInput.trim()}
                  className="w-full justify-center"
                  data-testid="entity-advanced-apply-bulk"
                >
                  {t('entityTypes.applyBulk', 'Apply Bulk List')}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => setAdvancedBulkInput(value.join('\n'))}
                  disabled={readOnly || value.length === 0}
                  className="w-full justify-center"
                  data-testid="entity-advanced-load-current"
                >
                  {t('entityTypes.loadCurrent', 'Load Current')}
                </Button>
              </div>
            </div>

            <div className="rounded-md border p-3 space-y-2">
              <p className="text-xs font-medium text-muted-foreground">
                {t('entityTypes.manage', 'Manage Current Types')}
              </p>
              <div className="grid gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={handleResetGeneral}
                  disabled={readOnly}
                  className="w-full justify-center"
                  data-testid="entity-advanced-reset-general"
                >
                  {t('entityTypes.resetGeneral', 'Reset to General')}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={handleSortAZ}
                  disabled={readOnly || value.length < 2}
                  className="w-full justify-center"
                  data-testid="entity-advanced-sort-az"
                >
                  {t('entityTypes.sortAZ', 'Sort A-Z')}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="destructive"
                  onClick={handleClearAll}
                  disabled={readOnly || value.length === 0}
                  className="w-full justify-center"
                  data-testid="entity-advanced-clear-all"
                >
                  {t('entityTypes.clearAll', 'Clear All')}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground" aria-live="polite">
                {t('entityTypes.currentCount', 'Current count')}: {value.length}/{MAX_ENTITY_TYPES}
              </p>
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default EntityTypeSelector;
