/**
 * Code Block Component
 * 
 * Renders code blocks with syntax highlighting using Shiki.
 * Includes copy-to-clipboard functionality and language badge.
 */
'use client';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Check, Copy, Download } from 'lucide-react';
import { memo, useCallback, useEffect, useState } from 'react';
import { bundledLanguages, codeToHtml, type BundledLanguage } from 'shiki';

interface CodeBlockProps {
  code: string;
  language?: string;
  className?: string;
  showLineNumbers?: boolean;
}

// Map common language aliases to Shiki language identifiers
const LANGUAGE_MAP: Record<string, BundledLanguage> = {
  'js': 'javascript',
  'ts': 'typescript',
  'tsx': 'tsx',
  'jsx': 'jsx',
  'py': 'python',
  'rb': 'ruby',
  'sh': 'bash',
  'shell': 'bash',
  'zsh': 'bash',
  'yml': 'yaml',
  'md': 'markdown',
  'rs': 'rust',
  'go': 'go',
  'java': 'java',
  'cpp': 'cpp',
  'c': 'c',
  'cs': 'csharp',
  'php': 'php',
  'sql': 'sql',
  'json': 'json',
  'html': 'html',
  'css': 'css',
  'scss': 'scss',
  'dockerfile': 'dockerfile',
  'docker': 'dockerfile',
  'graphql': 'graphql',
  'gql': 'graphql',
  'toml': 'toml',
  'diff': 'diff',
  'plaintext': 'text' as BundledLanguage,
  'text': 'text' as BundledLanguage,
  '': 'text' as BundledLanguage,
} as const;

function normalizeLanguage(lang: string | undefined): string {
  if (!lang) return 'text';
  const normalized = lang.toLowerCase().trim();
  const mapped = LANGUAGE_MAP[normalized as keyof typeof LANGUAGE_MAP] ?? normalized;

  // WHY: Shiki throws `ShikiError: Language 'X' is not included in this bundle`
  // for languages not in the bundle (e.g., "dafny", "verilog"). Validate against
  // the bundled languages map and fall back to plain text for unsupported ones.
  if (mapped in bundledLanguages) {
    return mapped;
  }
  return 'text';
}

export const CodeBlock = memo(function CodeBlock({
  code,
  language,
  className,
  showLineNumbers = false,
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [highlightedHtml, setHighlightedHtml] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const normalizedLang = normalizeLanguage(language);

  // Highlight code with Shiki
  useEffect(() => {
    let cancelled = false;

    async function highlight() {
      try {
        setIsLoading(true);
        const html = await codeToHtml(code, {
          lang: normalizedLang,
          theme: 'github-dark-dimmed',
        });
        if (!cancelled) {
          setHighlightedHtml(html);
        }
      } catch (error) {
        console.error('Shiki highlight error:', error);
        // Fallback to plain text on error
        if (!cancelled) {
          setHighlightedHtml(null);
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    }

    highlight();

    return () => {
      cancelled = true;
    };
  }, [code, normalizedLang]);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  }, [code]);

  const handleDownload = useCallback(() => {
    const blob = new Blob([code], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `code.${language || 'txt'}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [code, language]);

  return (
    <div
      className={cn(
        'group relative my-4 overflow-hidden rounded-lg border bg-zinc-900',
        className
      )}
    >
      {/* Header with language badge and actions */}
      <div className="flex items-center justify-between border-b border-zinc-700 bg-zinc-800 px-4 py-2">
        <span className="text-xs font-medium text-zinc-400 uppercase tracking-wider">
          {language || 'text'}
        </span>
        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-700"
            onClick={handleDownload}
            title="Download"
          >
            <Download className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 text-zinc-400 hover:text-zinc-100 hover:bg-zinc-700"
            onClick={handleCopy}
            title={copied ? 'Copied!' : 'Copy'}
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-400" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
        </div>
      </div>

      {/* Code content */}
      <div className="overflow-x-auto p-4">
        {isLoading ? (
          // Loading state - show plain code
          <pre className="text-sm text-zinc-300 font-mono whitespace-pre">
            <code>{code}</code>
          </pre>
        ) : highlightedHtml ? (
          // Shiki highlighted HTML
          <div
            className="text-sm [&_pre]:!bg-transparent [&_pre]:!p-0 [&_code]:text-sm"
            dangerouslySetInnerHTML={{ __html: highlightedHtml }}
          />
        ) : (
          // Fallback to plain text
          <pre className="text-sm text-zinc-300 font-mono whitespace-pre">
            <code>{code}</code>
          </pre>
        )}
      </div>
    </div>
  );
});

export default CodeBlock;
