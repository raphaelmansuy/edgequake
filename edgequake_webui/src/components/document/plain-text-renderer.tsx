// Plain text renderer with smart formatting
'use client';

import { Button } from '@/components/ui/button';
import { Copy } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

interface PlainTextRendererProps {
  content: string;
}

export function PlainTextRenderer({ content }: PlainTextRendererProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      toast.success('Content copied to clipboard');
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error('Failed to copy content');
    }
  };

  return (
    <div className="relative group">
      {/* Copy button */}
      <div className="absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity">
        <Button size="sm" variant="ghost" onClick={handleCopy}>
          <Copy className="h-3.5 w-3.5 mr-1.5" />
          {copied ? 'Copied!' : 'Copy'}
        </Button>
      </div>

      {/* Content */}
      <div className="bg-muted/30 rounded-xl p-6 border font-mono text-sm overflow-auto max-h-[70vh]">
        <pre className="whitespace-pre-wrap break-words text-foreground/90 leading-relaxed">
          {content}
        </pre>
      </div>
    </div>
  );
}
