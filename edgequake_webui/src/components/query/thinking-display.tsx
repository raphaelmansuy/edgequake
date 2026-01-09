/**
 * @module ThinkingDisplay
 * @description Chain-of-thought reasoning display component.
 * Shows LLM thinking process with collapsible sections.
 *
 * @implements FEAT0734 - Chain-of-thought display
 * @implements FEAT0750 - Collapsible thinking sections
 *
 * @enforces BR0105 - Thinking shows progressive indicators
 * @enforces BR0750 - Default collapsed for completed responses
 */
'use client';

import { cn } from '@/lib/utils';
import { Brain, ChevronDown, ChevronRight } from 'lucide-react';
import { memo, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface ThinkingDisplayProps {
  content: string;
  defaultExpanded?: boolean;
  className?: string;
}

interface ParsedContent {
  thinking: string[];
  response: string;
}

/**
 * Parse COT (Chain of Thought) content from LLM responses.
 * Supports multiple thinking block formats:
 * - <think>...</think>
 * - <thinking>...</thinking>
 * - **Thinking:**...
 */
export function parseCOTContent(content: string | undefined | null): ParsedContent {
  // Handle undefined/null content safely
  if (!content || typeof content !== 'string') {
    return { thinking: [], response: '' };
  }
  
  const thinking: string[] = [];
  let response = content;

  // Pattern 1: <think>...</think> tags
  const thinkTagRegex = /<think>([\s\S]*?)<\/think>/gi;
  let match;
  while ((match = thinkTagRegex.exec(content)) !== null) {
    thinking.push(match[1].trim());
  }
  response = response.replace(thinkTagRegex, '').trim();

  // Pattern 2: <thinking>...</thinking> tags
  const thinkingTagRegex = /<thinking>([\s\S]*?)<\/thinking>/gi;
  while ((match = thinkingTagRegex.exec(content)) !== null) {
    thinking.push(match[1].trim());
  }
  response = response.replace(thinkingTagRegex, '').trim();

  // Pattern 3: **Thinking:** block until next section
  const thinkingBlockRegex = /\*\*Thinking:\*\*\s*([\s\S]*?)(?=\n\n\*\*[A-Z]|\n\n---|\n\n#{1,3}\s|$)/gi;
  while ((match = thinkingBlockRegex.exec(content)) !== null) {
    thinking.push(match[1].trim());
  }
  response = response.replace(thinkingBlockRegex, '').trim();

  return {
    thinking,
    response,
  };
}

/**
 * Component to display LLM chain-of-thought reasoning in a collapsible section.
 */
export const ThinkingDisplay = memo(function ThinkingDisplay({
  content,
  defaultExpanded = false,
  className,
}: ThinkingDisplayProps) {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(defaultExpanded);

  const parsedContent = useMemo(() => parseCOTContent(content), [content]);

  // If no thinking content, just return the response without wrapper
  if (parsedContent.thinking.length === 0) {
    return null;
  }

  return (
    <div className={cn('rounded-lg border border-border bg-muted/50', className)}>
      {/* Collapsible thinking section */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center gap-2 w-full p-3 text-left hover:bg-muted/80 transition-colors rounded-t-lg"
      >
        {isExpanded ? (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 text-muted-foreground" />
        )}
        <Brain className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium text-muted-foreground">
          {t('query.thinking', 'Reasoning Process')}
        </span>
        <span className="text-xs text-muted-foreground/60 ml-auto">
          {parsedContent.thinking.length} {t('query.thinkingSteps', 'step(s)')}
        </span>
      </button>

      {/* Expanded thinking content */}
      {isExpanded && (
        <div className="p-3 pt-0 space-y-3">
          {parsedContent.thinking.map((block, index) => (
            <div
              key={index}
              className="pl-6 border-l-2 border-muted-foreground/30"
            >
              <p className="text-sm text-muted-foreground whitespace-pre-wrap">
                {block}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
});

/**
 * Wrapper component that renders both thinking and response sections.
 */
export const COTRenderer = memo(function COTRenderer({
  content,
  renderResponse,
  defaultThinkingExpanded = false,
  className,
}: {
  content: string;
  renderResponse: (response: string) => React.ReactNode;
  defaultThinkingExpanded?: boolean;
  className?: string;
}) {
  const parsedContent = useMemo(() => parseCOTContent(content), [content]);

  return (
    <div className={cn('space-y-4', className)}>
      {/* Thinking section (if any) */}
      {parsedContent.thinking.length > 0 && (
        <ThinkingDisplay
          content={content}
          defaultExpanded={defaultThinkingExpanded}
        />
      )}

      {/* Main response */}
      {parsedContent.response && renderResponse(parsedContent.response)}
    </div>
  );
});

export default ThinkingDisplay;
