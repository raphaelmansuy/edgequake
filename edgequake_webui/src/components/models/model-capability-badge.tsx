/**
 * @module ModelCapabilityBadge
 * @description Badge component to display model capabilities like vision, function calling, etc.
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - Model capability display
 * @iteration OODA #76 - Model Cards Component
 *
 * @enforces BR0305 - Users can see model capabilities before selection
 */
'use client';

import { Badge } from '@/components/ui/badge';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import type { ModelCapabilities } from '@/lib/api/models';
import { cn } from '@/lib/utils';
import {
    Braces,
    Eye,
    FileJson,
    MessageSquare,
    Sparkles,
    Zap,
} from 'lucide-react';

interface ModelCapabilityBadgeProps {
  capability: keyof ModelCapabilities | 'streaming' | 'system';
  enabled: boolean;
  showDisabled?: boolean;
  size?: 'sm' | 'md';
  className?: string;
}

/**
 * Get icon and label for a capability.
 */
function getCapabilityInfo(capability: string): {
  icon: React.ReactNode;
  label: string;
  description: string;
} {
  switch (capability) {
    case 'supports_vision':
      return {
        icon: <Eye className="h-3 w-3" />,
        label: 'Vision',
        description: 'Can analyze images and visual content',
      };
    case 'supports_function_calling':
      return {
        icon: <Braces className="h-3 w-3" />,
        label: 'Functions',
        description: 'Supports function/tool calling for structured outputs',
      };
    case 'supports_json_mode':
      return {
        icon: <FileJson className="h-3 w-3" />,
        label: 'JSON',
        description: 'Guarantees valid JSON output format',
      };
    case 'supports_streaming':
    case 'streaming':
      return {
        icon: <Zap className="h-3 w-3" />,
        label: 'Stream',
        description: 'Supports real-time streaming responses',
      };
    case 'supports_system_message':
    case 'system':
      return {
        icon: <MessageSquare className="h-3 w-3" />,
        label: 'System',
        description: 'Supports system message instructions',
      };
    default:
      return {
        icon: <Sparkles className="h-3 w-3" />,
        label: capability,
        description: `${capability} capability`,
      };
  }
}

/**
 * Single capability badge with tooltip.
 */
export function ModelCapabilityBadge({
  capability,
  enabled,
  showDisabled = false,
  size = 'sm',
  className,
}: ModelCapabilityBadgeProps) {
  const info = getCapabilityInfo(capability);

  if (!enabled && !showDisabled) {
    return null;
  }

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <Badge
            variant={enabled ? 'secondary' : 'outline'}
            className={cn(
              'gap-1 cursor-help',
              size === 'sm' ? 'px-1.5 py-0 text-xs' : 'px-2 py-0.5',
              !enabled && 'opacity-40',
              className
            )}
          >
            {info.icon}
            <span className={size === 'sm' ? 'sr-only sm:not-sr-only' : ''}>
              {info.label}
            </span>
          </Badge>
        </TooltipTrigger>
        <TooltipContent>
          <p className="text-sm font-medium">{info.label}</p>
          <p className="text-xs text-muted-foreground">{info.description}</p>
          <p className="text-xs mt-1">
            {enabled ? '✓ Supported' : '✗ Not supported'}
          </p>
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

interface ModelCapabilitiesDisplayProps {
  capabilities: ModelCapabilities;
  showAll?: boolean;
  size?: 'sm' | 'md';
  className?: string;
}

/**
 * Display all capabilities for a model.
 */
export function ModelCapabilitiesDisplay({
  capabilities,
  showAll = false,
  size = 'sm',
  className,
}: ModelCapabilitiesDisplayProps) {
  const caps = [
    { key: 'supports_vision', enabled: capabilities.supports_vision },
    { key: 'supports_function_calling', enabled: capabilities.supports_function_calling },
    { key: 'supports_json_mode', enabled: capabilities.supports_json_mode },
    { key: 'supports_streaming', enabled: capabilities.supports_streaming },
    { key: 'supports_system_message', enabled: capabilities.supports_system_message },
  ];

  const enabledCaps = caps.filter((c) => c.enabled);
  const displayCaps = showAll ? caps : enabledCaps;

  if (displayCaps.length === 0) {
    return null;
  }

  return (
    <div className={cn('flex flex-wrap gap-1', className)}>
      {displayCaps.map((cap) => (
        <ModelCapabilityBadge
          key={cap.key}
          capability={cap.key as keyof ModelCapabilities}
          enabled={cap.enabled}
          showDisabled={showAll}
          size={size}
        />
      ))}
    </div>
  );
}
