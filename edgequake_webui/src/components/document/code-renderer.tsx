// Code renderer with syntax highlighting and smart features
'use client';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { Check, Copy, Download } from 'lucide-react';
import { useTheme } from 'next-themes';
import { useState } from 'react';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark, oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism';
import { toast } from 'sonner';

interface CodeRendererProps {
  content: string;
  language: string;
  showLineNumbers?: boolean;
  fileName?: string;
}

export function CodeRenderer({
  content,
  language,
  showLineNumbers = true,
  fileName,
}: CodeRendererProps) {
  const [copied, setCopied] = useState(false);
  const { theme } = useTheme();

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast.success('Code copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error('Failed to copy code');
    }
  };

  const handleDownload = () => {
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName || `code.${getFileExtension(language)}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success('File downloaded');
  };

  const isDark = theme === 'dark';

  return (
    <div className="relative group">
      {/* Floating toolbar */}
      <div className="absolute top-3 right-3 z-10 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
        <div className="flex items-center gap-2 bg-background/95 backdrop-blur-sm border rounded-lg px-2 py-1.5 shadow-lg">
          <Badge variant="secondary" className="text-xs font-mono px-2 py-0.5">
            {language}
          </Badge>
          <Separator orientation="vertical" className="h-4" />
          <Button
            size="sm"
            variant="ghost"
            onClick={handleCopy}
            className="h-7 px-2 hover:bg-accent"
          >
            {copied ? (
              <Check className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <Copy className="h-3.5 w-3.5" />
            )}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleDownload}
            className="h-7 px-2 hover:bg-accent"
          >
            <Download className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Code content with syntax highlighting */}
      <div className="rounded-xl border overflow-hidden shadow-sm">
        <SyntaxHighlighter
          language={language}
          style={isDark ? oneDark : oneLight}
          showLineNumbers={showLineNumbers}
          wrapLines
          customStyle={{
            margin: 0,
            borderRadius: 0,
            fontSize: '0.875rem',
            lineHeight: '1.6',
            padding: '1.5rem',
            background: isDark ? 'rgba(40, 44, 52, 1)' : 'rgba(250, 250, 250, 1)',
          }}
          lineNumberStyle={{
            minWidth: '3em',
            paddingRight: '1em',
            color: isDark ? '#5c6370' : '#a0a1a7',
            userSelect: 'none',
            borderRight: `1px solid ${isDark ? '#282c34' : '#e5e7eb'}`,
            marginRight: '1em',
          }}
          codeTagProps={{
            style: {
              fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
            },
          }}
        >
          {content}
        </SyntaxHighlighter>
      </div>
    </div>
  );
}

function getFileExtension(language: string): string {
  const extensionMap: Record<string, string> = {
    javascript: 'js',
    typescript: 'ts',
    python: 'py',
    java: 'java',
    rust: 'rs',
    go: 'go',
    ruby: 'rb',
    php: 'php',
    sql: 'sql',
    bash: 'sh',
    yaml: 'yml',
    json: 'json',
    css: 'css',
    scss: 'scss',
    html: 'html',
    xml: 'xml',
    markdown: 'md',
    cpp: 'cpp',
    c: 'c',
  };
  return extensionMap[language] || 'txt';
}
