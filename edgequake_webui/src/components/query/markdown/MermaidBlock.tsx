/**
 * Mermaid Diagram Component
 * 
 * Lazy-loads and renders Mermaid diagrams with proper error handling.
 * Shows a placeholder during streaming to avoid parsing incomplete syntax.
 */
'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { AlertTriangle, GitBranch, RefreshCw } from 'lucide-react';
import { memo, useEffect, useId, useRef, useState } from 'react';

interface MermaidBlockProps {
  code: string;
  className?: string;
  isStreaming?: boolean;
}

// Mermaid is imported dynamically to reduce initial bundle size
let mermaidPromise: Promise<typeof import('mermaid')> | null = null;
let mermaidInstance: typeof import('mermaid').default | null = null;
let currentTheme: 'dark' | 'default' | null = null;

/**
 * Detect current theme from DOM.
 * Returns 'dark' if documentElement has .dark class, else 'default'.
 */
function detectTheme(): 'dark' | 'default' {
  if (typeof document === 'undefined') return 'dark';
  return document.documentElement.classList.contains('dark') ? 'dark' : 'default';
}

async function getMermaid(forceTheme?: 'dark' | 'default') {
  const theme = forceTheme ?? detectTheme();
  
  if (mermaidInstance && currentTheme === theme) return mermaidInstance;
  
  if (!mermaidPromise || currentTheme !== theme) {
    mermaidPromise = import('mermaid').then((mod) => {
      mermaidInstance = mod.default;
      currentTheme = theme;
      mermaidInstance.initialize({
        startOnLoad: false,
        theme,
        securityLevel: 'loose',
        fontFamily: 'ui-sans-serif, system-ui, sans-serif',
        themeVariables: theme === 'dark' ? {
          primaryColor: '#3b82f6',
          primaryTextColor: '#e4e4e7',
          primaryBorderColor: '#52525b',
          lineColor: '#71717a',
          secondaryColor: '#27272a',
          tertiaryColor: '#18181b',
          background: '#18181b',
          mainBkg: '#27272a',
          nodeBorder: '#52525b',
          clusterBkg: '#1e1e24',
          clusterBorder: '#3f3f46',
          titleColor: '#e4e4e7',
          edgeLabelBackground: '#27272a',
          nodeTextColor: '#e4e4e7',
        } : {
          primaryColor: '#3b82f6',
          primaryTextColor: '#18181b',
          primaryBorderColor: '#d4d4d8',
          lineColor: '#a1a1aa',
          secondaryColor: '#f4f4f5',
          tertiaryColor: '#fafafa',
          background: '#ffffff',
          mainBkg: '#f4f4f5',
          nodeBorder: '#d4d4d8',
          clusterBkg: '#fafafa',
          clusterBorder: '#e4e4e7',
          titleColor: '#18181b',
          edgeLabelBackground: '#f4f4f5',
          nodeTextColor: '#18181b',
        },
        flowchart: {
          htmlLabels: true,
          curve: 'basis',
        },
        sequence: {
          diagramMarginX: 50,
          diagramMarginY: 10,
          actorMargin: 50,
          width: 150,
          height: 65,
          boxMargin: 10,
          boxTextMargin: 5,
          noteMargin: 10,
          messageMargin: 35,
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
 */
function sanitizeMermaidCode(code: string): { sanitized: string; issues: string[] } {
  const issues: string[] = [];
  let sanitized = code.trim();

  // Remove markdown code block markers if present
  if (sanitized.startsWith('```')) {
    sanitized = sanitized.replace(/^```(?:mermaid)?\n?/, '').replace(/\n?```$/, '');
    issues.push('Removed code block markers');
  }

  // Fix common LLM issues with node IDs containing special characters
  // Mermaid node IDs should be alphanumeric with underscores
  const lines = sanitized.split('\n');
  const fixedLines = lines.map((line, index) => {
    // Skip the graph type declaration line
    if (index === 0 && /^(graph|flowchart|sequenceDiagram|classDiagram|stateDiagram|erDiagram|gantt|pie|gitGraph)/i.test(line.trim())) {
      return line;
    }
    
    // Fix node IDs with spaces in arrow definitions
    // e.g., "Node Name --> Other Node" should become "NodeName --> OtherNode"
    // But preserve text in brackets/quotes
    return line;
  });
  sanitized = fixedLines.join('\n');

  // Check for completely empty diagram
  const contentLines = sanitized.split('\n').filter(l => l.trim() && !l.trim().startsWith('%%'));
  if (contentLines.length < 2) {
    issues.push('Diagram appears incomplete (less than 2 content lines)');
  }

  return { sanitized, issues };
}

/**
 * Check if Mermaid code looks complete enough to attempt rendering.
 * This is a lightweight pre-check before invoking the full parser.
 */
function isProbablyValidMermaid(code: string): boolean {
  const trimmed = code.trim();
  
  // Must start with a valid diagram type
  const validStarts = [
    'graph', 'flowchart', 'sequenceDiagram', 'classDiagram', 
    'stateDiagram', 'erDiagram', 'gantt', 'pie', 'gitGraph',
    'journey', 'mindmap', 'timeline', 'sankey', 'block',
  ];
  
  const firstWord = trimmed.split(/[\s\n]/)[0].toLowerCase();
  if (!validStarts.some(start => firstWord.startsWith(start))) {
    return false;
  }

  // Must have at least some content after the declaration
  const lines = trimmed.split('\n').filter(l => l.trim() && !l.trim().startsWith('%%'));
  return lines.length >= 2;
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
  const [isLoading, setIsLoading] = useState(true);
  const [sanitizedCode, setSanitizedCode] = useState<string>(code);
  const [theme, setTheme] = useState<'dark' | 'default'>(() => detectTheme());

  // Listen for theme changes
  useEffect(() => {
    const observer = new MutationObserver(() => {
      const newTheme = detectTheme();
      setTheme(prev => {
        if (prev !== newTheme) {
          // Reset mermaid instance so it reinitializes with new theme
          mermaidPromise = null;
          mermaidInstance = null;
          currentTheme = null;
          return newTheme;
        }
        return prev;
      });
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
    return () => observer.disconnect();
  }, []);

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

        // Pre-validate and sanitize the code
        const { sanitized, issues } = sanitizeMermaidCode(code);
        setSanitizedCode(sanitized);
        
        if (issues.length > 0) {
          console.log('Mermaid code sanitization:', issues);
        }

        // Quick pre-check before loading Mermaid
        if (!isProbablyValidMermaid(sanitized)) {
          if (!cancelled) {
            setError('Invalid diagram format. Mermaid diagrams must start with a diagram type (graph, flowchart, sequenceDiagram, etc.)');
            setIsLoading(false);
          }
          return;
        }

        const mermaid = await getMermaid();
        
        // Try to parse first - this will throw if invalid
        try {
          await mermaid.parse(sanitized);
        } catch (parseError) {
          if (!cancelled) {
            const errorMsg = parseError instanceof Error ? parseError.message : 'Syntax error';
            throw new Error(`Parse error: ${errorMsg}`);
          }
          return;
        }

        // Render the diagram with sanitized code
        const { svg: renderedSvg } = await mermaid.render(
          `mermaid-${uniqueId}`,
          sanitized
        );

        if (!cancelled) {
          setSvg(renderedSvg);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          console.error('Mermaid render error:', err);
          setError(err instanceof Error ? err.message : 'Failed to render diagram');
          setSvg(null);
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    }

    renderDiagram();

    return () => {
      cancelled = true;
    };
  }, [code, isStreaming, uniqueId, theme]);

  const handleRetry = () => {
    setError(null);
    setIsLoading(true);
    // Trigger re-render by re-calling the effect
    setSvg(null);
  };

  // Streaming placeholder
  if (isStreaming) {
    return (
      <div
        className={cn(
          'my-4 flex items-center justify-center rounded-lg border border-dashed',
          'border-border bg-muted/30 p-8',
          className
        )}
        role="status"
        aria-label="Diagram loading"
      >
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <GitBranch className="h-8 w-8 animate-pulse" aria-hidden="true" />
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
          'my-4 flex items-center justify-center rounded-lg border border-border bg-muted/20 p-8',
          className
        )}
        role="status"
        aria-label="Rendering diagram"
      >
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <RefreshCw className="h-6 w-6 animate-spin" aria-hidden="true" />
          <span className="text-sm">Rendering diagram...</span>
        </div>
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div
        className={cn(
          'my-4 rounded-lg border border-destructive/50 bg-destructive/5 p-4',
          className
        )}
        role="alert"
      >
        <div className="flex items-start gap-3">
          <AlertTriangle className="h-5 w-5 text-destructive shrink-0 mt-0.5" aria-hidden="true" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-destructive">
              Failed to render Mermaid diagram
            </p>
            <p className="mt-1 text-xs text-destructive/70 wrap-break-word">{error}</p>
            <details className="mt-3">
              <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground transition-colors">
                Show source
              </summary>
              <pre className="mt-2 overflow-x-auto rounded bg-muted p-3 text-xs text-muted-foreground">
                <code>{code}</code>
              </pre>
            </details>
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive hover:bg-destructive/10"
            onClick={handleRetry}
            aria-label="Retry rendering diagram"
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </div>
    );
  }

  // Success - render the SVG
  if (svg) {
    return (
      <div
        ref={containerRef}
        className={cn(
          'my-4 overflow-x-auto rounded-lg border border-border bg-muted/20 p-4',
          '[&_svg]:mx-auto [&_svg]:max-w-full',
          className
        )}
        role="img"
        aria-label="Mermaid diagram"
        dangerouslySetInnerHTML={{ __html: svg }}
      />
    );
  }

  // Empty state
  return null;
});

export default MermaidBlock;
