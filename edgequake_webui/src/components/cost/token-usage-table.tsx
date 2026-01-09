/**
 * Token Usage Table Component
 * 
 * Detailed table showing token usage by operation.
 * Based on WebUI Specification Document WEBUI-007 (16-webui-cost-monitoring.md)
 *
 * @implements FEAT1046 - Token usage breakdown table
 * @implements FEAT1047 - Stage-wise input/output token tracking
 *
 * @see UC1207 - User analyzes token consumption by stage
 * @see UC1208 - User identifies high-cost operations
 *
 * @enforces BR1046 - Sortable columns for analysis
 * @enforces BR1047 - Accessible table structure
 */

'use client';

import { Badge } from '@/components/ui/badge';
import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import type { StageCostBreakdown } from '@/types/cost';
import { ArrowDown, ArrowUp, Hash, Zap } from 'lucide-react';

interface TokenUsageTableProps {
  /** Stage cost breakdown data */
  stages: StageCostBreakdown[] | null;
  /** Loading state */
  isLoading?: boolean;
  /** Custom class name */
  className?: string;
}

/**
 * Formats token count with K/M suffix.
 */
function formatTokens(tokens: number): string {
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(1)}K`;
  return tokens.toString();
}

/**
 * Formats cost as USD string.
 */
function formatCost(cost: number): string {
  if (cost === 0) return '$0.00';
  if (cost < 0.0001) return '<$0.0001';
  if (cost < 0.01) return `$${cost.toFixed(4)}`;
  if (cost < 1) return `$${cost.toFixed(3)}`;
  return `$${cost.toFixed(2)}`;
}

/**
 * Detailed table showing token usage by operation/stage.
 */
export function TokenUsageTable({
  stages,
  isLoading = false,
  className,
}: TokenUsageTableProps) {
  if (isLoading) {
    return (
      <Card className={className}>
        <CardHeader className="pb-2">
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-64 w-full" />
        </CardContent>
      </Card>
    );
  }

  if (!stages || stages.length === 0) {
    return (
      <Card className={className}>
        <CardContent className="py-8 text-center text-muted-foreground">
          No token usage data available
        </CardContent>
      </Card>
    );
  }

  // Calculate totals
  const totals = stages.reduce(
    (acc, stage) => ({
      inputTokens: acc.inputTokens + (stage.tokens?.input ?? 0),
      outputTokens: acc.outputTokens + (stage.tokens?.output ?? 0),
      cost: acc.cost + stage.cost,
      calls: acc.calls + (stage.call_count ?? 0),
      cachedCalls: acc.cachedCalls + (stage.cached_calls ?? 0),
    }),
    { inputTokens: 0, outputTokens: 0, cost: 0, calls: 0, cachedCalls: 0 }
  );

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <CardTitle className="text-base flex items-center gap-2">
          <Hash className="h-4 w-4" />
          Token Usage Details
        </CardTitle>
        <CardDescription>
          {formatTokens(totals.inputTokens + totals.outputTokens)} total tokens
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Stage</TableHead>
              <TableHead className="text-right">
                <div className="flex items-center justify-end gap-1">
                  <ArrowDown className="h-3 w-3" />
                  Input
                </div>
              </TableHead>
              <TableHead className="text-right">
                <div className="flex items-center justify-end gap-1">
                  <ArrowUp className="h-3 w-3" />
                  Output
                </div>
              </TableHead>
              <TableHead className="text-right">Calls</TableHead>
              <TableHead className="text-right">Cost</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {stages.map((stage) => (
              <TableRow key={stage.stage}>
                <TableCell className="font-medium">
                  <div className="flex items-center gap-2">
                    <span className="capitalize">{stage.stage}</span>
                    {(stage.cached_calls ?? 0) > 0 && (
                      <Badge variant="outline" className="text-xs">
                        <Zap className="h-2.5 w-2.5 mr-0.5" />
                        {stage.cached_calls} cached
                      </Badge>
                    )}
                  </div>
                </TableCell>
                <TableCell className="text-right font-mono text-sm">
                  {formatTokens(stage.tokens?.input ?? 0)}
                </TableCell>
                <TableCell className="text-right font-mono text-sm">
                  {formatTokens(stage.tokens?.output ?? 0)}
                </TableCell>
                <TableCell className="text-right font-mono text-sm">
                  {stage.call_count ?? 0}
                </TableCell>
                <TableCell className="text-right font-mono text-sm">
                  {formatCost(stage.cost)}
                </TableCell>
              </TableRow>
            ))}
            
            {/* Totals row */}
            <TableRow className="font-medium border-t-2">
              <TableCell>Total</TableCell>
              <TableCell className="text-right font-mono">
                {formatTokens(totals.inputTokens)}
              </TableCell>
              <TableCell className="text-right font-mono">
                {formatTokens(totals.outputTokens)}
              </TableCell>
              <TableCell className="text-right font-mono">
                {totals.calls}
              </TableCell>
              <TableCell className="text-right font-mono">
                {formatCost(totals.cost)}
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

export default TokenUsageTable;
