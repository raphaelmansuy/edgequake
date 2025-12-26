// Smart content renderer that adapts to document MIME type
'use client';

import { MarkdownRenderer } from '@/components/query/markdown-renderer';
import { Skeleton } from '@/components/ui/skeleton';
import type { Document } from '@/types';
import { Suspense, useMemo } from 'react';
import { CodeRenderer } from './code-renderer';
import { PlainTextRenderer } from './plain-text-renderer';

interface ContentRendererProps {
  document: Document;
}

export function ContentRenderer({ document }: ContentRendererProps) {
  const renderer = useMemo(() => {
    return getRendererForDocument(document);
  }, [document]);

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <Suspense fallback={<ContentSkeleton />}>
        {renderer}
      </Suspense>
    </div>
  );
}

function getRendererForDocument(doc: Document) {
  const mimeType = doc.mime_type?.toLowerCase() || '';
  const fileName = doc.file_name?.toLowerCase() || '';
  const content = doc.content || doc.content_summary || '';

  // Markdown documents
  if (
    isMarkdown(mimeType) ||
    fileName.endsWith('.md') ||
    fileName.endsWith('.markdown') ||
    hasMarkdownSignature(content)
  ) {
    return (
      <article className="
        prose prose-lg dark:prose-invert max-w-none
        prose-headings:font-display prose-headings:font-semibold
        prose-h1:text-4xl prose-h1:mb-6 prose-h1:mt-8
        prose-h2:text-3xl prose-h2:mb-4 prose-h2:mt-6
        prose-h3:text-2xl prose-h3:mb-3 prose-h3:mt-5
        prose-p:text-base prose-p:leading-relaxed prose-p:text-foreground/90
        prose-a:text-primary prose-a:no-underline prose-a:font-medium
        hover:prose-a:underline
        prose-code:bg-muted prose-code:px-1.5 prose-code:py-0.5 
        prose-code:rounded prose-code:text-sm prose-code:font-mono
        prose-code:before:content-none prose-code:after:content-none
        prose-pre:bg-muted/50 prose-pre:border prose-pre:rounded-xl
        prose-pre:p-4 prose-pre:overflow-x-auto
        prose-blockquote:border-l-4 prose-blockquote:border-primary
        prose-blockquote:bg-muted/30 prose-blockquote:py-2 prose-blockquote:px-4
        prose-blockquote:rounded-r-lg prose-blockquote:italic
        prose-img:rounded-xl prose-img:shadow-lg
        prose-hr:border-border prose-hr:my-8
        prose-table:border prose-table:rounded-lg
        prose-thead:bg-muted
      ">
        <MarkdownRenderer
          content={content}
          enableMath={true}
          enableMermaid={true}
          className="text-sm leading-relaxed"
        />
      </article>
    );
  }

  // Code files
  if (isCode(mimeType, fileName)) {
    const language = detectLanguage(mimeType, fileName);
    return (
      <CodeRenderer
        content={content}
        language={language}
        showLineNumbers
      />
    );
  }

  // JSON/Structured data
  if (mimeType === 'application/json' || fileName.endsWith('.json')) {
    try {
      const parsed = JSON.parse(content);
      return (
        <CodeRenderer
          content={JSON.stringify(parsed, null, 2)}
          language="json"
          showLineNumbers
        />
      );
    } catch {
      // Fall through to plain text if JSON parsing fails
    }
  }

  // Fallback: Plain text with smart formatting
  return <PlainTextRenderer content={content} />;
}

// Helper functions
function isMarkdown(mimeType: string): boolean {
  return (
    mimeType.includes('markdown') ||
    mimeType === 'text/markdown' ||
    mimeType === 'text/x-markdown'
  );
}

function hasMarkdownSignature(content: string): boolean {
  if (!content) return false;
  // Check for common markdown patterns
  const markdownPatterns = [
    /^#{1,6}\s+/m,        // Headers
    /\*\*[^*]+\*\*/,      // Bold
    /\*[^*]+\*/,          // Italic
    /\[[^\]]+\]\([^)]+\)/, // Links
    /```[\s\S]*```/,      // Code blocks
    /^\s*[-*+]\s+/m,      // Lists
  ];
  return markdownPatterns.some((pattern) => pattern.test(content));
}

function isCode(mimeType: string, fileName: string): boolean {
  const codeMimeTypes = [
    'text/x-python',
    'text/x-java',
    'text/x-c',
    'text/x-c++',
    'text/javascript',
    'application/javascript',
    'text/typescript',
    'application/typescript',
    'text/x-rust',
    'text/x-go',
    'text/x-ruby',
    'text/x-php',
    'text/x-sql',
    'text/x-sh',
    'text/x-yaml',
    'application/x-yaml',
    'text/css',
    'text/html',
    'application/xml',
    'text/xml',
  ];

  const codeExtensions = [
    '.py', '.js', '.ts', '.tsx', '.jsx', '.java', '.c', '.cpp', '.h', '.hpp',
    '.rs', '.go', '.rb', '.php', '.sql', '.sh', '.bash', '.zsh', '.yaml', '.yml',
    '.css', '.scss', '.sass', '.less', '.html', '.xml', '.toml', '.ini', '.conf',
  ];

  return (
    codeMimeTypes.some((type) => mimeType.includes(type)) ||
    codeExtensions.some((ext) => fileName.endsWith(ext))
  );
}

function detectLanguage(mimeType: string, fileName: string): string {
  // Language mapping
  const mimeToLang: Record<string, string> = {
    'text/x-python': 'python',
    'text/x-java': 'java',
    'text/javascript': 'javascript',
    'application/javascript': 'javascript',
    'text/typescript': 'typescript',
    'application/typescript': 'typescript',
    'text/x-rust': 'rust',
    'text/x-go': 'go',
    'text/x-ruby': 'ruby',
    'text/x-php': 'php',
    'text/x-sql': 'sql',
    'text/x-sh': 'bash',
    'text/x-yaml': 'yaml',
    'application/x-yaml': 'yaml',
    'text/css': 'css',
    'text/html': 'html',
    'application/xml': 'xml',
    'text/xml': 'xml',
  };

  // Try MIME type first
  for (const [mime, lang] of Object.entries(mimeToLang)) {
    if (mimeType.includes(mime)) {
      return lang;
    }
  }

  // Fall back to file extension
  const ext = fileName.split('.').pop()?.toLowerCase();
  const extToLang: Record<string, string> = {
    py: 'python',
    js: 'javascript',
    ts: 'typescript',
    tsx: 'typescript',
    jsx: 'javascript',
    java: 'java',
    c: 'c',
    cpp: 'cpp',
    h: 'c',
    hpp: 'cpp',
    rs: 'rust',
    go: 'go',
    rb: 'ruby',
    php: 'php',
    sql: 'sql',
    sh: 'bash',
    bash: 'bash',
    zsh: 'bash',
    yaml: 'yaml',
    yml: 'yaml',
    css: 'css',
    scss: 'scss',
    sass: 'sass',
    less: 'less',
    html: 'html',
    xml: 'xml',
    json: 'json',
    toml: 'toml',
    ini: 'ini',
    conf: 'bash',
  };

  return ext && extToLang[ext] ? extToLang[ext] : 'text';
}

function ContentSkeleton() {
  return (
    <div className="space-y-4">
      <Skeleton className="h-8 w-3/4" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-5/6" />
      <Skeleton className="h-32 w-full mt-6" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-4/5" />
    </div>
  );
}
