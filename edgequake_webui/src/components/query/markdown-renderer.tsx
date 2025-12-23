'use client';

import { memo, useEffect, useState } from 'react';
import type { Components } from 'react-markdown';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import remarkGfm from 'remark-gfm';
import remarkMath from 'remark-math';

// Import KaTeX CSS dynamically in useEffect to avoid SSR issues
// This is handled in the component below

// Lazy load rehype-katex since it's heavy
let rehypeKatexModule: typeof import('rehype-katex').default | null = null;
let katexCssLoaded = false;

interface MarkdownRendererProps {
  content: string;
  className?: string;
  enableMath?: boolean;
  enableMermaid?: boolean;
}

/**
 * Enhanced Markdown renderer with LaTeX math support.
 * Uses KaTeX for rendering mathematical formulas.
 */
export const MarkdownRenderer = memo(function MarkdownRenderer({
  content,
  className = '',
  enableMath = true,
  enableMermaid = true,
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

  // Build rehype plugins array
  const rehypePlugins: import('unified').PluggableList = [rehypeHighlight];
  if (enableMath && rehypeKatex) {
    rehypePlugins.push(rehypeKatex);
  }

  // Build remark plugins array
  const remarkPlugins: import('unified').PluggableList = [remarkGfm];
  if (enableMath) {
    remarkPlugins.push(remarkMath);
  }

  // Custom components for code blocks (for Mermaid detection)
  const components: Components = {
    // Handle code blocks
    code({ className: codeClassName, children, ...props }) {
      const match = /language-(\w+)/.exec(codeClassName || '');
      const language = match ? match[1] : '';
      const codeContent = String(children).replace(/\n$/, '');

      // Check for Mermaid diagram
      if (enableMermaid && language === 'mermaid') {
        return <MermaidDiagram code={codeContent} />;
      }

      // Regular code block
      return (
        <code className={codeClassName} {...props}>
          {children}
        </code>
      );
    },
  };

  return (
    <div className={`prose prose-sm dark:prose-invert max-w-none ${className}`}>
      <ReactMarkdown
        remarkPlugins={remarkPlugins}
        rehypePlugins={rehypePlugins}
        components={components}
      >
        {content}
      </ReactMarkdown>
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
