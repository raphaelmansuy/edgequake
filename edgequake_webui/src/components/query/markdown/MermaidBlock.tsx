/**
 * Mermaid Diagram Component
 * 
 * Lazy-loads and renders Mermaid diagrams with proper error handling.
 * Supports light/dark themes via next-themes.
 * Includes a full-view dialog for large diagrams.
 * Shows a placeholder during streaming to avoid parsing incomplete syntax.
 */
'use client';

import { Button } from '@/components/ui/button';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from '@/components/ui/dialog';
import { cn } from '@/lib/utils';
import { GitBranch, Maximize2, RefreshCw } from 'lucide-react';
import { useTheme } from 'next-themes';
import { memo, useEffect, useId, useRef, useState } from 'react';

interface MermaidBlockProps {
  code: string;
  className?: string;
  isStreaming?: boolean;
}

// Mermaid is imported dynamically to reduce initial bundle size
let mermaidPromise: Promise<typeof import('mermaid')> | null = null;
let mermaidInstance: typeof import('mermaid').default | null = null;
let currentMermaidTheme: string | null = null;

async function getMermaid(isDark: boolean) {
  const desiredTheme = isDark ? 'dark' : 'default';
  
  // If mermaid is already loaded but theme changed, re-initialize
  if (mermaidInstance && currentMermaidTheme !== desiredTheme) {
    mermaidInstance.initialize({
      startOnLoad: false,
      theme: desiredTheme,
      securityLevel: 'loose',
      fontFamily: 'ui-sans-serif, system-ui, sans-serif',
      flowchart: { htmlLabels: true, curve: 'basis' },
      sequence: {
        diagramMarginX: 50, diagramMarginY: 10, actorMargin: 50,
        width: 150, height: 65, boxMargin: 10, boxTextMargin: 5,
        noteMargin: 10, messageMargin: 35,
      },
    });
    currentMermaidTheme = desiredTheme;
    return mermaidInstance;
  }
  
  if (mermaidInstance) return mermaidInstance;
  
  if (!mermaidPromise) {
    mermaidPromise = import('mermaid').then((mod) => {
      mermaidInstance = mod.default;
      currentMermaidTheme = desiredTheme;
      mermaidInstance.initialize({
        startOnLoad: false,
        theme: desiredTheme,
        securityLevel: 'loose',
        fontFamily: 'ui-sans-serif, system-ui, sans-serif',
        flowchart: { htmlLabels: true, curve: 'basis' },
        sequence: {
          diagramMarginX: 50, diagramMarginY: 10, actorMargin: 50,
          width: 150, height: 65, boxMargin: 10, boxTextMargin: 5,
          noteMargin: 10, messageMargin: 35,
        },
      });
      return mod;
    });
  }
  
  const mod = await mermaidPromise;
  return mod.default;
}

/**
 * Pre-validate and sanitize Mermaid code to fix common LLM output issues.
 * Returns sanitized code and any detected issues.
 *
 * WHY: LLMs frequently generate Mermaid syntax that is semantically correct
 * but syntactically invalid. The most common issues are:
 * 1. Parentheses inside brackets: `A[text (note)]` — Mermaid interprets `(` as shape delimiter
 * 2. Unicode characters in node IDs: `动作模型[label]` — must be ASCII IDs
 * 3. Unquoted labels with special chars: pipes, braces, forward slashes, backslashes, etc.
 * 4. Bare curly-brace expressions `{label}` with no preceding node ID → invalid DIAMOND_START
 * 5. Forward slashes `/` or backslashes `\` inside node labels without quoting
 *
 * The fix: wrap label text in double quotes when it contains problematic characters.
 * Mermaid supports `A["text with (parens) and 中文"]` syntax.
 *
 * Exported for unit-testing.
 */
export function sanitizeMermaidCode(code: string): { sanitized: string; issues: string[] } {
  const issues: string[] = [];
  let sanitized = code.trim();

  // Remove markdown code block markers if present
  if (sanitized.startsWith('```')) {
    sanitized = sanitized.replace(/^```(?:mermaid)?\n?/, '').replace(/\n?```$/, '');
    issues.push('Removed code block markers');
  }

  // Strip HTML tags from node labels.
  // WHY: LLMs frequently emit <br>, <br/>, <b>, <i> etc. inside Mermaid node labels.
  // Mermaid's tokeniser treats `<` as a shape delimiter (asymmetric node shape),
  // so `<br>` in a label causes a parse error even with htmlLabels:true.
  // We strip all HTML tags here; Mermaid's own \n handling is sufficient for layout.
  if (/<[a-zA-Z][^>]*>|<\/[a-zA-Z]+>/.test(sanitized)) {
    sanitized = sanitized.replace(/<br\s*\/?>/gi, ' ')  // <br> → space (most common)
                          .replace(/<[^>]+>/g, '');     // strip any remaining HTML tags
    issues.push('Stripped HTML tags from diagram source');
  }

  const lines = sanitized.split('\n');
  let bareNodeCounter = 0;
  const fixedLines = lines.map((line) => {
    const trimmed = line.trim();

    // Skip empty lines, comments, diagram type declarations, and subgraph/end/style keywords
    if (
      !trimmed ||
      trimmed.startsWith('%%') ||
      /^(graph|flowchart|sequenceDiagram|classDiagram|stateDiagram|erDiagram|gantt|pie|gitGraph|journey|mindmap|timeline|sankey|block)\b/i.test(trimmed) ||
      /^(subgraph|end|style|classDef|click|linkStyle|direction)\b/i.test(trimmed)
    ) {
      return line;
    }

    // Fix node definitions with bracket-style labels that contain special characters.
    // Matches patterns like: NodeId[label text] or NodeId[label (with parens)]
    // Character class includes: parens, braces, pipes, angle brackets, slashes,
    // @, &, #, : (common in email addresses and URLs), and Unicode.
    // WHY: @ triggers LINK_ID token; & triggers AMP; # and : cause parser confusion.
    let processedLine = line.replace(
      /([A-Za-z0-9_\u4e00-\u9fff\u3400-\u4dbf]+)\[([^\]"]*[(){}|><\/\\@&#:\u4e00-\u9fff\u3400-\u4dbf\u3000-\u303f\uff00-\uffef][^\]"]*)\]/g,
      (_match, nodeId: string, labelText: string) => {
        // If the label already has quotes, leave it alone
        if (labelText.startsWith('"') && labelText.endsWith('"')) return _match;

        // Escape any internal double quotes in the label
        const escaped = labelText.replace(/"/g, '#quot;');
        issues.push(`Quoted label: ${nodeId}["${escaped}"]`);
        return `${nodeId}["${escaped}"]`;
      }
    );

    // Fix rhombus/diamond nodes with special chars: NodeId{label/with/slashes}
    // Mermaid `A{label}` is a valid rhombus but the label must be quoted if it
    // contains `/`, `\`, `|`, `<`, `>`, `@`, `&`, `#`, `:` or Unicode.
    processedLine = processedLine.replace(
      /([A-Za-z0-9_]+)\{([^}"]*[/\\|<>@&#:\u4e00-\u9fff\u3400-\u4dbf][^}"]*)\}/g,
      (_match, nodeId: string, labelText: string) => {
        if (labelText.startsWith('"') && labelText.endsWith('"')) return _match;
        const escaped = labelText.replace(/"/g, '#quot;');
        issues.push(`Quoted rhombus label: ${nodeId}{"${escaped}"}`);
        return `${nodeId}{"${escaped}"}`;
      }
    );

    // Fix bare curly-brace expressions that have no preceding node ID.
    // LLMs often emit `A --> {label}` meaning "a node labelled label",
    // but Mermaid expects `A --> NodeId` or `A --> NodeId["label"]`.
    // e.g. `People --> {Personnes/Gens}` → `People --> _bare_1["Personnes/Gens"]`
    // Use a counter scoped to the outer map to produce stable unique IDs.
    processedLine = processedLine.replace(
      /(?<![A-Za-z0-9_"])\{([^}]+)\}/g,
      (_match, content: string) => {
        bareNodeCounter++;
        const escaped = content.replace(/"/g, '#quot;');
        issues.push(`Fixed bare curly-brace node: {${content}} → _bare_${bareNodeCounter}["${escaped}"]`);
        return `_bare_${bareNodeCounter}["${escaped}"]`;
      }
    );

    // Fix round-bracket (stadium) node labels that contain special characters.
    // e.g. `A(user@domain.com)` → `A("user@domain.com")`
    // WHY: `@`, `&`, `|`, `#`, `:`, `<`, `>`, `/`, `\` inside `()` labels
    // cause Mermaid parse errors because the lexer interprets them as tokens.
    processedLine = processedLine.replace(
      /([A-Za-z0-9_]+)\(([^()"]*[@&|#:<>/\\][^()"]*)\)/g,
      (_match, nodeId: string, labelText: string) => {
        if (labelText.startsWith('"') && labelText.endsWith('"')) return _match;
        const escaped = labelText.replace(/"/g, '#quot;');
        issues.push(`Quoted round-bracket label: ${nodeId}("${escaped}")`);
        return `${nodeId}("${escaped}")`;
      }
    );

    return processedLine;
  });
  sanitized = fixedLines.join('\n');

  // Fix node IDs that contain non-ASCII characters (e.g., Chinese)
  // Convert them to ASCII IDs while preserving the label.
  // e.g., `动作模型 --> 其他` becomes `node_1["动作模型"] --> node_2["其他"]`
  // Only fix standalone non-ASCII IDs in arrow definitions (not already in brackets).
  let nodeCounter = 0;
  const nodeIdMap = new Map<string, string>();

  sanitized = sanitized.replace(
    // Match non-ASCII word appearing in arrow context (not inside brackets)
    /(?<=^|\s|-->|---|-\.->|==>|-.->|~~>|--?>)[\s]*([\u4e00-\u9fff\u3400-\u4dbf\u3000-\u303f\uff00-\uffef][\w\u4e00-\u9fff\u3400-\u4dbf\u3000-\u303f\uff00-\uffef]*)[\s]*(?=$|\s|-->|---|-\.->|==>|-.->|~~>|--?>)/gm,
    (_match, unicodeId: string) => {
      if (!nodeIdMap.has(unicodeId)) {
        nodeCounter++;
        nodeIdMap.set(unicodeId, `node_${nodeCounter}`);
      }
      const asciiId = nodeIdMap.get(unicodeId)!;
      issues.push(`Mapped non-ASCII node ID: ${unicodeId} → ${asciiId}`);
      return `${asciiId}["${unicodeId}"]`;
    }
  );

  // Check for completely empty diagram
  const contentLines = sanitized.split('\n').filter(l => l.trim() && !l.trim().startsWith('%%'));
  if (contentLines.length < 2) {
    issues.push('Diagram appears incomplete (less than 2 content lines)');
  }

  return { sanitized, issues };
}

/**
 * MermaidFallback — renders Mermaid source as a styled code block.
 *
 * Single Responsibility: display diagram syntax as readable text when SVG
 * rendering is impossible. Never shows a red error panel; always preserves
 * the diagram's information content so the user can read or copy it.
 *
 * WHY (First Principle): a diagram that can't be rendered still carries
 * its full information in source form. Hiding it behind an error panel
 * destroys that information; a code block exposes it.
 */
function MermaidFallback({
  code,
  className,
  onRetry,
}: {
  code: string;
  className?: string;
  onRetry?: () => void;
}) {
  // Strip markdown fences — the plain source is what we display.
  let source = code.trim();
  if (source.startsWith('```')) {
    source = source.replace(/^```(?:mermaid)?\n?/, '').replace(/\n?```$/, '').trim();
  }
  return (
    <div
      className={cn('my-4 rounded-lg border border-border overflow-hidden', className)}
      role="figure"
      aria-label="Mermaid diagram source"
    >
      {/* Header bar — minimal, non-alarming */}
      <div className="flex items-center justify-between gap-2 px-4 py-2 bg-muted/60 border-b border-border/60">
        <div className="flex items-center gap-2">
          <GitBranch className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
          <span className="text-xs text-muted-foreground font-medium">Mermaid diagram (source)</span>
        </div>
        {onRetry && (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 w-6 p-0 text-muted-foreground hover:text-foreground"
            onClick={onRetry}
            aria-label="Retry rendering diagram"
            title="Retry"
          >
            <RefreshCw className="h-3.5 w-3.5" aria-hidden="true" />
          </Button>
        )}
      </div>
      {/* Source code — plain, selectable, scrollable */}
      <pre className="overflow-x-auto p-4 text-xs leading-relaxed text-foreground/80 bg-muted/20 whitespace-pre">
        <code>{source}</code>
      </pre>
    </div>
  );
}

export const MermaidBlock = memo(function MermaidBlock({
  code,
  className,
  isStreaming = false,
}: MermaidBlockProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const uniqueId = useId().replace(/:/g, '-');
  const [svg, setSvg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sanitizedCode, setSanitizedCode] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isFullView, setIsFullView] = useState(false);
  const { resolvedTheme } = useTheme();

  const isDark = resolvedTheme === 'dark';

  useEffect(() => {
    // Don't render while streaming - mermaid syntax is often incomplete
    if (isStreaming) {
      setIsLoading(true);
      setSvg(null);
      setError(null);
      return;
    }

    let cancelled = false;

    async function renderDiagram() {
      if (!code.trim()) {
        setIsLoading(false);
        return;
      }

      try {
        setIsLoading(true);
        setError(null);

        // Load mermaid first — the parser is the single source of truth for validity.
        // WHY: We must not gate rendering on heuristic string checks. Only
        // mermaid.parse() can correctly determine whether a diagram is valid.
        const mermaid = await getMermaid(isDark);

        // Step 1: strip markdown code fence — this is safe pre-processing, not a
        // heuristic transform: a fenced code block is never valid Mermaid syntax.
        let stripped = code.trim();
        if (stripped.startsWith('```')) {
          stripped = stripped.replace(/^```(?:mermaid)?\n?/, '').replace(/\n?```$/, '').trim();
        }

        // Step 2: try the fence-stripped original through the real parser.
        let codeToRender = stripped;
        try {
          await mermaid.parse(stripped);
          // Parser accepted the original — no further transformation needed.
        } catch {
          // Step 3: original rejected — apply the sanitizer heuristics as a
          // targeted fallback for known LLM output patterns.
          const { sanitized, issues } = sanitizeMermaidCode(code);
          setSanitizedCode(sanitized);
          if (issues.length > 0) {
            console.log('Mermaid sanitization applied:', issues);
          }
          // Validate the sanitized version using the same authoritative parser.
          // If this also throws, the outer catch block handles the error state.
          await mermaid.parse(sanitized);
          codeToRender = sanitized;
        }

        // Render only code that the parser has already accepted.
        const { svg: renderedSvg } = await mermaid.render(
          `mermaid-${uniqueId}-${isDark ? 'd' : 'l'}`,
          codeToRender
        );

        if (!cancelled) {
          setSvg(renderedSvg);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          // WHY: console.error() in Next.js dev mode triggers the global error overlay
          // even when the error is caught and handled gracefully. We use console.debug
          // so the info is visible in DevTools without polluting the overlay.
          console.debug('[mermaid] graceful fallback:', err instanceof Error ? err.message : err);
          setError(err instanceof Error ? err.message : 'Failed to render diagram');
          setSvg(null);
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    }

    // WHY: .catch(()=>{}) prevents an unhandled-promise-rejection if renderDiagram
    // throws synchronously after being awaited (e.g., mermaid import failure).
    // All intentional error paths are already handled inside the function.
    renderDiagram().catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [code, isStreaming, uniqueId, isDark]);

  const handleRetry = () => {
    setError(null);
    setIsLoading(true);
    setSvg(null);
  };

  // Streaming placeholder
  if (isStreaming) {
    return (
      <div
        className={cn(
          'my-4 flex items-center justify-center rounded-lg border border-dashed p-8',
          'border-border/60 bg-muted/30',
          className
        )}
        role="status"
        aria-label="Diagram loading"
      >
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <GitBranch className="h-8 w-8 motion-safe:animate-pulse" aria-hidden="true" />
          <span className="text-sm">Diagram loading...</span>
        </div>
      </div>
    );
  }

  // Loading state
  if (isLoading) {
    return (
      <div
        className={cn(
          'my-4 flex items-center justify-center rounded-lg border border-border bg-muted/40 dark:bg-zinc-900 p-8',
          className
        )}
        role="status"
        aria-label="Rendering diagram"
      >
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <RefreshCw className="h-6 w-6 motion-safe:animate-spin" aria-hidden="true" />
          <span className="text-sm">Rendering diagram...</span>
        </div>
      </div>
    );
  }

  // Fallback: when Mermaid parsing fails even after sanitization, display the source
  // as a styled code block instead of an error panel.
  //
  // WHY (First Principle): The diagram's information content lives in its source.
  // A red error panel blocks that content; a code block preserves it.
  // SOLID-SRP: MermaidFallback owns exactly one concern — rendering Mermaid as text.
  if (error) {
    return <MermaidFallback code={code} className={className} onRetry={handleRetry} />;
  }

  // Success - render the SVG with full-view button
  if (svg) {
    return (
      <>
        <div
          ref={containerRef}
          className={cn(
            'group relative my-4 overflow-x-auto rounded-lg border border-border p-4',
            'bg-muted/40 dark:bg-zinc-900',
            '[&_svg]:mx-auto [&_svg]:max-w-full',
            className
          )}
        >
          {/* Floating full-view button */}
          <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-muted-foreground hover:text-foreground hover:bg-accent"
              onClick={() => setIsFullView(true)}
              title="Full view"
              aria-label="Expand diagram to full view"
            >
              <Maximize2 className="h-3.5 w-3.5" aria-hidden="true" />
            </Button>
          </div>
          <div
            dangerouslySetInnerHTML={{ __html: svg }}
            role="img"
            aria-label="Mermaid diagram"
          />
        </div>

        {/* Full-view dialog */}
        <Dialog open={isFullView} onOpenChange={setIsFullView}>
          <DialogContent className="max-w-[90vw] w-full max-h-[90vh] flex flex-col">
            <DialogHeader>
              <DialogTitle className="text-sm font-mono uppercase tracking-wider">
                Mermaid Diagram
              </DialogTitle>
            </DialogHeader>
            <div
              className={cn(
                'flex-1 overflow-auto rounded-lg border border-border p-6',
                'bg-muted/40 dark:bg-zinc-900',
                '[&_svg]:mx-auto [&_svg]:max-w-full'
              )}
              dangerouslySetInnerHTML={{ __html: svg }}
              role="img"
              aria-label="Mermaid diagram (expanded)"
            />
          </DialogContent>
        </Dialog>
      </>
    );
  }

  // Empty state
  return null;
});

export default MermaidBlock;
