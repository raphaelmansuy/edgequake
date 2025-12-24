'use client';

import { cn } from '@/lib/utils';
import { Check, Copy } from 'lucide-react';
import { Component, ErrorInfo, memo, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { Components } from 'react-markdown';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';

// Import KaTeX CSS dynamically in useEffect to avoid SSR issues
let rehypeKatexModule: typeof import('rehype-katex').default | null = null;
let katexCssLoaded = false;

interface MarkdownRendererProps {
  content: string;
  className?: string;
  enableMath?: boolean;
  enableMermaid?: boolean;
  isStreaming?: boolean;
}

/**
 * Error boundary to catch react-markdown errors gracefully.
 * This handles edge cases where props might be undefined during streaming.
 */
class MarkdownErrorBoundary extends Component<
  { children: ReactNode; fallback: ReactNode },
  { hasError: boolean }
> {
  constructor(props: { children: ReactNode; fallback: ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(): { hasError: boolean } {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // Log the error but don't crash
    console.warn('MarkdownRenderer error caught:', error.message);
  }

  componentDidUpdate(prevProps: { children: ReactNode; fallback: ReactNode }): void {
    // Reset error state when children change (new content)
    if (prevProps.children !== this.props.children && this.state.hasError) {
      this.setState({ hasError: false });
    }
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return this.props.fallback;
    }
    return this.props.children;
  }
}

/**
 * Safe wrapper for react-markdown component props.
 * Returns null if props is undefined to prevent "Cannot use 'in' operator" error.
 */
function safeProps<T>(props: T | undefined): T | null {
  if (props === undefined || props === null) {
    return null;
  }
  return props;
}

/**
 * Enhanced Markdown renderer with LaTeX math support.
 * Uses KaTeX for rendering mathematical formulas.
 * 
 * Compatible with react-markdown v10+
 */
export const MarkdownRenderer = memo(function MarkdownRenderer({
  content,
  className = '',
  enableMath = true,
  enableMermaid = true,
  isStreaming = false,
}: MarkdownRendererProps) {
  const [rehypeKatex, setRehypeKatex] = useState<typeof import('rehype-katex').default | null>(
    rehypeKatexModule
  );

  // Lazy load rehype-katex and CSS on first render if math is enabled
  useEffect(() => {
    if (enableMath && !rehypeKatexModule) {
      // Load KaTeX CSS
      if (!katexCssLoaded && typeof document !== 'undefined') {
        const link = document.createElement('link');
        link.rel = 'stylesheet';
        link.href = 'https://cdn.jsdelivr.net/npm/katex@0.16.27/dist/katex.min.css';
        link.crossOrigin = 'anonymous';
        document.head.appendChild(link);
        katexCssLoaded = true;
      }
      
      import('rehype-katex').then((module) => {
        rehypeKatexModule = module.default;
        setRehypeKatex(() => module.default);
      });
    }
  }, [enableMath]);

  // Use refs for values that shouldn't cause component recreation
  const isStreamingRef = useRef(isStreaming);
  const enableMermaidRef = useRef(enableMermaid);
  
  // Keep refs updated
  useEffect(() => {
    isStreamingRef.current = isStreaming;
  }, [isStreaming]);
  
  useEffect(() => {
    enableMermaidRef.current = enableMermaid;
  }, [enableMermaid]);

  // Build rehype plugins array - disable KaTeX during streaming
  const rehypePlugins = useMemo(() => {
    const plugins: import('unified').PluggableList = [rehypeHighlight];
    // Only enable KaTeX when not streaming (partial LaTeX breaks parsing)
    if (enableMath && rehypeKatex && !isStreaming) {
      plugins.push(rehypeKatex);
    }
    return plugins;
  }, [enableMath, rehypeKatex, isStreaming]);

  // Build remark plugins array - disable math during streaming to prevent parsing errors
  const remarkPlugins = useMemo(() => {
    const plugins: import('unified').PluggableList = [remarkGfm];
    // Only enable math processing when not streaming (partial LaTeX breaks parsing)
    if (enableMath && !isStreaming) {
      plugins.push(remarkMath);
    }
    return plugins;
  }, [enableMath, isStreaming]);

  // Custom components for code blocks (react-markdown v10+ compatible)
  // Using stable reference pattern - components don't depend on isStreaming/enableMermaid directly
  // Instead they read from refs to avoid recreation
  const components: Components = useMemo(() => ({
    // Handle code blocks - react-markdown v10 passes node in ExtraProps
    code(props) {
      // Safely check for undefined/null props - critical for preventing the "Cannot use 'in' operator" error
      if (props === undefined || props === null) {
        return null;
      }
      
      // Safely try to destructure - wrapped in try-catch as extra safety
      try {
        const { className: codeClassName, children, ...rest } = props;
        
        // Safely extract children as string - handle undefined/null cases
        if (children === undefined || children === null) {
          return null;
        }
        const childContent = String(children);
        const codeContent = childContent.replace(/\n$/, '');
        
        // Extract language from className
        const match = /language-(\w+)/.exec(codeClassName || '');
        const language = match ? match[1] : '';

        // Check for Mermaid diagram (only if not streaming to prevent partial renders)
        // Read from ref instead of closure to avoid component recreation
        if (enableMermaidRef.current && language === 'mermaid' && !isStreamingRef.current) {
          return <MermaidDiagram code={codeContent} />;
        }

        // Check if this is a code block (has language) vs inline code
        const isBlock = !!language;

        // For code blocks, wrap in a styled container with copy button
        if (isBlock) {
          return (
            <CodeBlock language={language} code={codeContent}>
              <code className={codeClassName} {...rest}>
                {children}
              </code>
            </CodeBlock>
          );
        }

        // Inline code
        return (
          <code className={cn('bg-muted px-1 py-0.5 rounded text-sm', codeClassName)} {...rest}>
            {children}
          </code>
        );
      } catch {
        // If any error occurs, return null to prevent crash
        return null;
      }
    },
    // Better paragraph handling - ensure children exists with null check
    p(props) {
      if (props === undefined || props === null) return null;
      return <p className="my-2 leading-7">{props.children ?? null}</p>;
    },
    // Headings with null checks
    h1(props) {
      if (props === undefined || props === null) return null;
      return <h1 className="text-xl font-bold mt-4 mb-2">{props.children ?? null}</h1>;
    },
    h2(props) {
      if (props === undefined || props === null) return null;
      return <h2 className="text-lg font-bold mt-4 mb-2">{props.children ?? null}</h2>;
    },
    h3(props) {
      if (props === undefined || props === null) return null;
      return <h3 className="text-base font-bold mt-3 mb-2">{props.children ?? null}</h3>;
    },
    // Lists with null checks
    ul(props) {
      if (props === undefined || props === null) return null;
      return <ul className="list-disc pl-5 my-2 space-y-1">{props.children ?? null}</ul>;
    },
    ol(props) {
      if (props === undefined || props === null) return null;
      return <ol className="list-decimal pl-5 my-2 space-y-1">{props.children ?? null}</ol>;
    },
    li(props) {
      if (props === undefined || props === null) return null;
      return <li className="my-1">{props.children ?? null}</li>;
    },
    // Blockquotes with null check
    blockquote(props) {
      if (props === undefined || props === null) return null;
      return (
        <blockquote className="border-l-4 border-primary/30 pl-4 my-3 italic text-muted-foreground">
          {props.children ?? null}
        </blockquote>
      );
    },
    // Tables with null checks
    table(props) {
      if (props === undefined || props === null) return null;
      return (
        <div className="my-4 overflow-x-auto">
          <table className="min-w-full border-collapse border border-border">{props.children ?? null}</table>
        </div>
      );
    },
    th(props) {
      if (props === undefined || props === null) return null;
      return <th className="border border-border bg-muted px-3 py-2 text-left font-semibold">{props.children ?? null}</th>;
    },
    td(props) {
      if (props === undefined || props === null) return null;
      return <td className="border border-border px-3 py-2">{props.children ?? null}</td>;
    },
  // Empty dependency array - components use refs for dynamic values
  }), []);

  // Don't render empty or undefined content
  if (!content || typeof content !== 'string') {
    return null;
  }

  // Normalize markdown syntax that may be broken by streaming tokenization
  // Streaming often adds extra spaces around markdown markers like ** for bold
  // e.g., "** AI Model **" should be "**AI Model**" for proper rendering
  const normalizeMarkdown = (text: string): string => {
    return text
      // Fix bold: "** text **" -> "**text**"
      .replace(/\*\*\s+/g, '**')
      .replace(/\s+\*\*/g, '**')
      // Fix italic (single asterisk): "* text *" -> "*text*" - be careful with list items
      // Only fix when there's a matching pair, not list items at start of line
      .replace(/(\*)\s+(\S)/g, (match, star, char) => {
        // Don't fix if it looks like a list item (start of line)
        return `${star}${char}`;
      })
      .replace(/(\S)\s+(\*)/g, '$1$2')
      // Fix code: "` text `" -> "`text`"
      .replace(/`\s+/g, '`')
      .replace(/\s+`/g, '`')
      // Fix strikethrough: "~~ text ~~" -> "~~text~~"
      .replace(/~~\s+/g, '~~')
      .replace(/\s+~~/g, '~~');
  };

  // Sanitize content to prevent parsing errors
  const safeContent = normalizeMarkdown(content.trim());
  if (!safeContent) {
    return null;
  }

  // Generate a stable key based on content length to help React reconciliation
  // This helps prevent errors during rapid content updates
  const contentKey = `md-${safeContent.length}-${safeContent.slice(0, 20).replace(/\W/g, '')}`;

  // Fallback content for when error occurs during streaming
  const fallback = (
    <div className={cn('prose prose-sm dark:prose-invert max-w-none break-words', className)}>
      <p className="whitespace-pre-wrap break-words">{safeContent}</p>
    </div>
  );

  // During active streaming with partial content, use fallback to avoid parsing errors
  // Once streaming is done or content is stable, use full markdown rendering
  if (isStreaming && safeContent.length < 50) {
    return fallback;
  }

  return (
    <MarkdownErrorBoundary key={contentKey} fallback={fallback}>
      <div className={cn('prose prose-sm dark:prose-invert max-w-none break-words', className)}>
        <ReactMarkdown
          remarkPlugins={remarkPlugins}
          rehypePlugins={rehypePlugins}
          components={components}
        >
          {safeContent}
        </ReactMarkdown>
      </div>
    </MarkdownErrorBoundary>
  );
});

/**
 * Code block with copy button
 */
const CodeBlock = memo(function CodeBlock({
  language,
  code,
  children,
}: {
  language: string;
  code: string;
  children: ReactNode;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  }, [code]);

  return (
    <div className="relative group my-4">
      <div className="absolute top-2 right-2 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
        <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">
          {language}
        </span>
        <button
          onClick={handleCopy}
          className="p-1 rounded bg-muted hover:bg-muted/80 transition-colors"
          title="Copy code"
        >
          {copied ? (
            <Check className="h-3.5 w-3.5 text-green-500" />
          ) : (
            <Copy className="h-3.5 w-3.5 text-muted-foreground" />
          )}
        </button>
      </div>
      <pre className="overflow-x-auto rounded-lg bg-muted/50 p-4 text-sm">
        {children}
      </pre>
    </div>
  );
});

/**
 * Mermaid diagram component with lazy loading
 */
const MermaidDiagram = memo(function MermaidDiagram({ code }: { code: string }) {
  const [svg, setSvg] = useState<string>('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    const renderDiagram = async () => {
      try {
        // Dynamic import of mermaid
        const { default: mermaid } = await import('mermaid');

        mermaid.initialize({
          startOnLoad: false,
          theme: document.documentElement.classList.contains('dark') ? 'dark' : 'default',
          securityLevel: 'loose',
        });

        const id = `mermaid-${Math.random().toString(36).substr(2, 9)}`;
        const { svg: renderedSvg } = await mermaid.render(id, code);

        if (isMounted) {
          setSvg(renderedSvg);
          setError(null);
        }
      } catch (err) {
        console.error('Mermaid render error:', err);
        if (isMounted) {
          setError(err instanceof Error ? err.message : 'Failed to render diagram');
        }
      }
    };

    if (code) {
      renderDiagram();
    }

    return () => {
      isMounted = false;
    };
  }, [code]);

  if (error) {
    return (
      <div className="my-4 p-4 border border-red-300 bg-red-50 dark:bg-red-900/20 rounded-lg">
        <p className="text-sm text-red-600 dark:text-red-400">
          Failed to render Mermaid diagram: {error}
        </p>
        <pre className="mt-2 text-xs overflow-x-auto">
          <code>{code}</code>
        </pre>
      </div>
    );
  }

  if (!svg) {
    return (
      <div className="my-4 p-4 bg-muted rounded-lg animate-pulse">
        <div className="h-32 bg-muted-foreground/10 rounded" />
      </div>
    );
  }

  return (
    <div
      className="my-4 overflow-x-auto"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
});

export default MarkdownRenderer;
