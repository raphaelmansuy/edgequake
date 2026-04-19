'use client';

/**
 * Config Explainability Panel
 *
 * Shows the FULL priority chain for LLM / Embedding / Vision configuration
 * so that operators can immediately see:
 *   - Which level is active (the winning value)
 *   - Where it came from (env var name, DB field, compiled default)
 *   - Whether there is a provider/model mismatch warning
 *
 * Uses GET /api/v1/config/effective  (no auth required — read-only diagnostics)
 *
 * Design principle: First Principles / No Flaky Config
 *   Every displayed value is derived deterministically from the same resolution
 *   chain the server uses at startup. There is no hidden state.
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { apiClient } from '@/lib/api/client';
import {
    AlertTriangle,
    CheckCircle2,
    ChevronDown,
    ChevronRight,
    Info,
    RefreshCw,
    Settings2,
    Zap,
} from 'lucide-react';
import { useEffect, useState } from 'react';

// ── Types ────────────────────────────────────────────────────────────────────

interface ConfigLevel {
  level: string;
  label: string;
  provider: string | null;
  model: string | null;
  active: boolean;
  note: string | null;
  source: string | null;
}

interface ConfigAreaResponse {
  levels: ConfigLevel[];
  effective_provider: string;
  effective_model: string;
  has_mismatch: boolean;
  mismatch_description: string | null;
}

interface EffectiveConfigResponse {
  llm: ConfigAreaResponse;
  embedding: ConfigAreaResponse;
  vision: ConfigAreaResponse;
  priority_rule: string;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const AREA_META: Record<string, { title: string; icon: React.ReactNode; description: string }> = {
  llm: {
    title: 'LLM (Chat & Extraction)',
    icon: <Zap className="h-4 w-4" />,
    description: 'Used for chat completions and entity extraction',
  },
  embedding: {
    title: 'Embedding',
    icon: <Settings2 className="h-4 w-4" />,
    description: 'Used to encode documents and queries into vector space',
  },
  vision: {
    title: 'Vision / PDF',
    icon: <Info className="h-4 w-4" />,
    description: 'Used to read pages of uploaded PDF documents',
  },
};

function levelBadgeVariant(level: string): 'default' | 'secondary' | 'outline' | 'destructive' {
  switch (level) {
    case 'compiled_default': return 'outline';
    case 'env_alias': return 'secondary';
    case 'env_secondary': return 'secondary';
    case 'env_primary': return 'default';
    case 'env_llm_inherit': return 'secondary';
    case 'env_vision': return 'default';
    default: return 'outline';
  }
}

// ── Sub-components ────────────────────────────────────────────────────────────

function ConfigAreaSection({
  areaKey,
  area,
}: {
  areaKey: string;
  area: ConfigAreaResponse;
}) {
  const [expanded, setExpanded] = useState(area.has_mismatch); // auto-expand if mismatch
  const meta = AREA_META[areaKey] ?? { title: areaKey, icon: null, description: '' };

  return (
    <div className="rounded-lg border bg-card">
      {/* Header row — always visible */}
      <button
        className="flex w-full items-center justify-between px-4 py-3 text-left hover:bg-muted/50 transition-colors rounded-lg"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
      >
        <div className="flex items-center gap-3">
          <span className="text-muted-foreground">{meta.icon}</span>
          <div>
            <span className="font-medium text-sm">{meta.title}</span>
            <p className="text-xs text-muted-foreground mt-0.5">{meta.description}</p>
          </div>
        </div>

        <div className="flex items-center gap-3">
          {/* Effective values pill */}
          <div className="hidden sm:flex items-center gap-1.5 text-xs bg-muted rounded-md px-2 py-1">
            <span className="text-muted-foreground">provider</span>
            <span className="font-mono font-semibold">{area.effective_provider}</span>
            <span className="text-muted-foreground mx-0.5">·</span>
            <span className="text-muted-foreground">model</span>
            <span className="font-mono font-semibold">{area.effective_model}</span>
          </div>

          {/* Mismatch warning */}
          {area.has_mismatch && (
            <AlertTriangle className="h-4 w-4 text-amber-500 shrink-0" />
          )}

          {/* Expand icon */}
          {expanded
            ? <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" />
            : <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />}
        </div>
      </button>

      {/* Expanded detail */}
      {expanded && (
        <div className="border-t px-4 pb-4 pt-3 space-y-3">
          {/* Mobile: show effective values again */}
          <div className="sm:hidden flex gap-2 text-xs">
            <span className="text-muted-foreground">Effective:</span>
            <span className="font-mono font-semibold">{area.effective_provider} / {area.effective_model}</span>
          </div>

          {/* Mismatch banner with remediation guidance */}
          {area.has_mismatch && area.mismatch_description && (
            <div className="rounded-md border border-amber-200 bg-amber-50 dark:bg-amber-950/30 dark:border-amber-800 px-3 py-2 space-y-2">
              <div className="flex items-start gap-2">
                <AlertTriangle className="h-4 w-4 text-amber-500 mt-0.5 shrink-0" />
                <p className="text-xs font-semibold text-amber-700 dark:text-amber-400">Configuration Mismatch Detected</p>
              </div>
              <div className="text-xs text-amber-600 dark:text-amber-500 space-y-1.5 ml-6">
                {area.mismatch_description.split('\n').filter(Boolean).map((line, i) => {
                  const trimmed = line.trim();
                  if (trimmed.startsWith('•')) {
                    return (
                      <div key={i} className="flex items-start gap-1.5 pl-2">
                        <span className="shrink-0 mt-0.5">•</span>
                        <span className="font-mono text-[11px]">{trimmed.slice(1).trim()}</span>
                      </div>
                    );
                  }
                  if (trimmed.startsWith('How to fix')) {
                    return <p key={i} className="font-semibold mt-1">{trimmed}</p>;
                  }
                  return <p key={i}>{trimmed}</p>;
                })}
              </div>
            </div>
          )}

          {/* Priority chain table */}
          <div className="text-xs text-muted-foreground font-medium uppercase tracking-wide mb-1">
            Resolution Chain (lowest → highest priority)
          </div>
          <div className="space-y-1.5">
            {area.levels.map((lvl, idx) => (
              <div
                key={idx}
                className={`flex items-start gap-3 rounded-md px-3 py-2 border transition-colors ${
                  lvl.active
                    ? 'border-primary/40 bg-primary/5 dark:bg-primary/10'
                    : 'border-transparent bg-muted/30'
                }`}
              >
                {/* Active indicator */}
                <div className="mt-0.5 shrink-0">
                  {lvl.active ? (
                    <CheckCircle2 className="h-3.5 w-3.5 text-primary" />
                  ) : (
                    <div className="h-3.5 w-3.5 rounded-full border-2 border-muted-foreground/30" />
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex flex-wrap items-center gap-1.5 mb-1">
                    <Badge variant={levelBadgeVariant(lvl.level)} className="text-[10px] py-0 h-4">
                      {lvl.label}
                    </Badge>
                    {lvl.active && (
                      <Badge variant="default" className="text-[10px] py-0 h-4 bg-primary">
                        ACTIVE
                      </Badge>
                    )}
                  </div>

                  {/* Values */}
                  {lvl.provider || lvl.model ? (
                    <div className="flex flex-wrap gap-x-4 gap-y-0.5 text-xs">
                      {lvl.provider && (
                        <span>
                          <span className="text-muted-foreground">provider: </span>
                          <span className="font-mono">{lvl.provider}</span>
                        </span>
                      )}
                      {lvl.model && (
                        <span>
                          <span className="text-muted-foreground">model: </span>
                          <span className="font-mono">{lvl.model}</span>
                        </span>
                      )}
                    </div>
                  ) : (
                    <span className="text-xs text-muted-foreground italic">Not set at this level</span>
                  )}

                  {/* Source */}
                  {lvl.source && (
                    <p className="text-[10px] text-muted-foreground/70 mt-0.5 font-mono">
                      source: {lvl.source}
                    </p>
                  )}

                  {/* Note */}
                  {lvl.note && (
                    <p className="text-[10px] text-muted-foreground mt-0.5">{lvl.note}</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Main Component ────────────────────────────────────────────────────────────

export function ConfigExplainabilityPanel() {
  const [config, setConfig] = useState<EffectiveConfigResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  const fetchConfig = async () => {
    try {
      setLoading(true);
      const data = await apiClient<EffectiveConfigResponse>('/config/effective');
      setConfig(data);
      setError(null);
      setLastRefresh(new Date());
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to load config';
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchConfig();
  }, []);

  const anyMismatch = config && (
    config.llm.has_mismatch || config.embedding.has_mismatch || config.vision.has_mismatch
  );

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Settings2 className="h-5 w-5" />
              Configuration Explainability
              {anyMismatch && (
                <AlertTriangle className="h-4 w-4 text-amber-500" />
              )}
            </CardTitle>
            <CardDescription className="mt-1">
              Full priority chain for every config area — see exactly which value wins and where it comes from.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={fetchConfig}
            disabled={loading}
            className="shrink-0"
          >
            <RefreshCw className={`h-3.5 w-3.5 mr-1.5 ${loading ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        </div>
      </CardHeader>

      <CardContent className="space-y-3">
        {/* Error state */}
        {error && (
          <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2">
            <AlertTriangle className="h-4 w-4 text-destructive mt-0.5 shrink-0" />
            <div>
              <p className="text-xs font-semibold text-destructive">Failed to load config</p>
              <p className="text-xs text-destructive/80 mt-0.5">{error}</p>
            </div>
          </div>
        )}

        {/* Loading state */}
        {loading && !config && (
          <div className="text-sm text-muted-foreground animate-pulse py-4 text-center">
            Loading configuration chain…
          </div>
        )}

        {/* Config areas */}
        {config && (
          <>
            <ConfigAreaSection areaKey="llm" area={config.llm} />
            <ConfigAreaSection areaKey="embedding" area={config.embedding} />
            <ConfigAreaSection areaKey="vision" area={config.vision} />

            <Separator />

            {/* Priority rule */}
            <div className="flex items-start gap-2 text-xs text-muted-foreground">
              <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
              <span>{config.priority_rule}</span>
            </div>

            {/* Last refresh */}
            <p className="text-[10px] text-muted-foreground/60 text-right">
              Last refreshed: {lastRefresh.toLocaleTimeString()}
            </p>
          </>
        )}
      </CardContent>
    </Card>
  );
}
