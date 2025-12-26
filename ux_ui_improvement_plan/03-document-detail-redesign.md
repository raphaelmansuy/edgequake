# Document Detail Page Redesign - Slick Design

**Priority:** HIGH  
**Estimated Effort:** 5-7 days  
**Complexity:** High

## Problem Analysis

### Current Issues (from screenshot and code review)

1. **Poor Scrolling Experience**
   - Fixed header takes too much vertical space
   - ScrollArea implementation causes nested scrolling issues
   - No sticky metadata cards during scroll
   - Lineage section buried at bottom

2. **Generic Content Display**
   - All content types rendered identically
   - No specialized viewers for markdown, code, PDF, images
   - Poor syntax highlighting for code content
   - No MIME type-aware rendering

3. **Information Architecture**
   - Metadata scattered across multiple cards
   - Lineage information hidden in collapsed section
   - No visual hierarchy for important vs. auxiliary info
   - Extraction data not prominent enough

4. **Visual Design**
   - Boxy, utilitarian appearance
   - Lack of breathing room and whitespace
   - No delightful micro-interactions
   - Cards feel heavy and disconnected

## Design Vision: Slick Document Viewer

### Core Principles

```
┌─────────────────────────────────────────────────────────────┐
│  SLEEK HEADER                            [Actions]           │
│  Document Title • Status • Quick Info                        │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────────────┐  ┌─────────────────────────────┐ │
│  │  CONTENT PREVIEW     │  │  SMART METADATA SIDEBAR     │ │
│  │  (Adaptive Renderer) │  │  • Key Stats (sticky)       │ │
│  │                      │  │  • Lineage Tree             │ │
│  │  - Markdown w/       │  │  • Entity/Relation Graph    │ │
│  │    katex + mermaid   │  │  • Extraction Timeline      │ │
│  │  - Code with syntax  │  │  • Source Info              │ │
│  │  - PDF preview       │  │                             │ │
│  │  - Image gallery     │  │  (Collapsible sections)     │ │
│  │  - JSON tree viewer  │  │                             │ │
│  │                      │  │                             │ │
│  │  (Smooth scrolling)  │  │  (Independent scroll)       │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Layout Strategy

#### 1. **Two-Column Layout with Intelligent Sidebar**

```tsx
// Adaptive layout based on content type and screen size
<div className="flex h-screen">
  {/* Main Content Area - 65% width on desktop */}
  <main className="flex-1 min-w-0">
    <ContentRenderer document={document} />
  </main>
  
  {/* Sticky Metadata Sidebar - 35% width on desktop */}
  <aside className="w-[35%] border-l">
    <MetadataSidebar document={document} />
  </aside>
</div>
```

#### 2. **Compact Header with Context**

```tsx
// Slim header with essential info
<header className="sticky top-0 z-50 border-b bg-background/95 backdrop-blur">
  <div className="flex items-center gap-4 p-3">
    <BackButton />
    <DocumentIcon type={document.mime_type} />
    <div className="flex-1 min-w-0">
      <h1 className="text-lg font-semibold truncate">{document.title}</h1>
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <StatusBadge status={document.status} />
        <span>•</span>
        <span>{formatFileSize(document.file_size)}</span>
        <span>•</span>
        <span>{formatDistanceToNow(document.created_at)}</span>
      </div>
    </div>
    <DocumentActions document={document} />
  </div>
</header>
```

## Content Renderer System

### MIME Type-Based Rendering

```typescript
// Smart content renderer that adapts to document type
export function ContentRenderer({ document }: { document: Document }) {
  const renderer = useMemo(() => {
    // Detect content type and return appropriate renderer
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

function getRendererForDocument(doc: Document): ReactNode {
  // Markdown documents
  if (isMarkdown(doc.mime_type) || hasMarkdownSignature(doc.content)) {
    return <MarkdownRenderer content={doc.content} enhanced />;
  }
  
  // Code files
  if (isCode(doc.mime_type)) {
    return <CodeRenderer 
      content={doc.content}
      language={detectLanguage(doc.mime_type, doc.file_name)}
      showLineNumbers
      theme="github-dark"
    />;
  }
  
  // PDF documents
  if (doc.mime_type === 'application/pdf') {
    return <PDFViewer url={doc.url} />;
  }
  
  // Images
  if (isImage(doc.mime_type)) {
    return <ImageViewer src={doc.url} alt={doc.title} />;
  }
  
  // JSON/Structured data
  if (doc.mime_type === 'application/json') {
    return <JSONTreeViewer data={JSON.parse(doc.content)} />;
  }
  
  // Fallback: Plain text with smart formatting
  return <PlainTextRenderer content={doc.content} />;
}
```

### Enhanced Markdown Renderer

```tsx
// components/document/enhanced-markdown-renderer.tsx
export function EnhancedMarkdownRenderer({ content }: { content: string }) {
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
      <ReactMarkdown
        remarkPlugins={[
          remarkGfm,           // GitHub Flavored Markdown
          remarkMath,          // Math support
          remarkToc,           // Table of contents
        ]}
        rehypePlugins={[
          rehypeKatex,         // Math rendering
          rehypeSlug,          // Add IDs to headings
          rehypeAutolinkHeadings, // Add links to headings
          [rehypePrettyCode, {  // Beautiful code blocks
            theme: 'github-dark',
            onVisitLine(node) {
              if (node.children.length === 0) {
                node.children = [{ type: 'text', value: ' ' }];
              }
            },
          }],
        ]}
        components={{
          // Custom component rendering
          code: CodeBlock,
          a: ExternalLink,
          img: LazyImage,
          table: ResponsiveTable,
          // Mermaid diagrams
          div: ({ node, className, children, ...props }) => {
            if (className === 'mermaid') {
              return <MermaidDiagram chart={children} />;
            }
            return <div className={className} {...props}>{children}</div>;
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </article>
  );
}
```

### Code Renderer with Smart Features

```tsx
// components/document/code-renderer.tsx
export function CodeRenderer({ 
  content, 
  language, 
  showLineNumbers = true,
  theme = 'github-dark'
}: CodeRendererProps) {
  const [copied, setCopied] = useState(false);
  
  return (
    <div className="relative group">
      {/* Floating toolbar */}
      <div className="absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity">
        <div className="flex items-center gap-2 bg-background/95 backdrop-blur border rounded-lg px-2 py-1 shadow-lg">
          <Badge variant="secondary" className="text-xs font-mono">
            {language}
          </Badge>
          <Separator orientation="vertical" className="h-4" />
          <Button 
            size="sm" 
            variant="ghost"
            onClick={() => handleCopy(content)}
            className="h-7 px-2"
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
            className="h-7 px-2"
          >
            <Download className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
      
      {/* Code content with syntax highlighting */}
      <SyntaxHighlighter
        language={language}
        style={theme === 'github-dark' ? githubDark : githubLight}
        showLineNumbers={showLineNumbers}
        wrapLines
        customStyle={{
          margin: 0,
          borderRadius: '0.75rem',
          fontSize: '0.875rem',
          lineHeight: '1.6',
          padding: '1.5rem',
        }}
        lineNumberStyle={{
          minWidth: '3em',
          paddingRight: '1em',
          color: 'var(--muted-foreground)',
          userSelect: 'none',
        }}
      >
        {content}
      </SyntaxHighlighter>
    </div>
  );
}
```

## Smart Metadata Sidebar

### Design Features

1. **Sticky Stats Card** - Always visible key metrics
2. **Collapsible Sections** - Progressive disclosure
3. **Visual Lineage Tree** - Interactive relationship view
4. **Entity Graph Preview** - Mini graph visualization
5. **Extraction Timeline** - Processing steps visualization

```tsx
// components/document/metadata-sidebar.tsx
export function MetadataSidebar({ document }: { document: Document }) {
  return (
    <div className="h-full flex flex-col">
      {/* Sticky Stats - Always visible */}
      <div className="sticky top-0 z-10 bg-background border-b p-4">
        <KeyStats document={document} />
      </div>
      
      {/* Scrollable sections */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {/* Extraction Lineage */}
          <CollapsibleSection
            title="Extraction Lineage"
            icon={<Brain className="h-4 w-4" />}
            defaultOpen
          >
            <LineageTree lineage={document.lineage} />
            <ExtractionTimeline lineage={document.lineage} />
          </CollapsibleSection>
          
          {/* Entity & Relationships */}
          <CollapsibleSection
            title="Knowledge Graph"
            icon={<Network className="h-4 w-4" />}
            defaultOpen
          >
            <MiniGraphPreview documentId={document.id} />
            <EntityRelationStats
              entities={document.entity_count}
              relationships={document.relationship_count}
            />
          </CollapsibleSection>
          
          {/* Source Information */}
          <CollapsibleSection
            title="Source Details"
            icon={<FileText className="h-4 w-4" />}
          >
            <SourceInfoGrid document={document} />
          </CollapsibleSection>
          
          {/* Processing Details */}
          <CollapsibleSection
            title="Processing Info"
            icon={<Settings className="h-4 w-4" />}
          >
            <ProcessingDetails lineage={document.lineage} />
          </CollapsibleSection>
        </div>
      </ScrollArea>
    </div>
  );
}
```

### Key Stats Card (Sticky)

```tsx
// Always visible stats at top of sidebar
function KeyStats({ document }: { document: Document }) {
  return (
    <div className="grid grid-cols-2 gap-3">
      <StatCard
        icon={<FileText className="h-4 w-4" />}
        label="Chunks"
        value={document.chunk_count}
        color="blue"
      />
      <StatCard
        icon={<Network className="h-4 w-4" />}
        label="Entities"
        value={document.entity_count}
        color="purple"
      />
      <StatCard
        icon={<Link2 className="h-4 w-4" />}
        label="Relations"
        value={document.relationship_count}
        color="green"
      />
      <StatCard
        icon={<Clock className="h-4 w-4" />}
        label="Processed"
        value={formatDuration(document.lineage?.processing_duration_ms)}
        color="orange"
      />
    </div>
  );
}

function StatCard({ icon, label, value, color }: StatCardProps) {
  const colorClasses = {
    blue: 'bg-blue-500/10 text-blue-600 dark:text-blue-400',
    purple: 'bg-purple-500/10 text-purple-600 dark:text-purple-400',
    green: 'bg-green-500/10 text-green-600 dark:text-green-400',
    orange: 'bg-orange-500/10 text-orange-600 dark:text-orange-400',
  };
  
  return (
    <div className="flex flex-col gap-1 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors">
      <div className={cn('flex items-center gap-1.5 text-xs font-medium', colorClasses[color])}>
        {icon}
        <span>{label}</span>
      </div>
      <div className="text-2xl font-bold">{value ?? '-'}</div>
    </div>
  );
}
```

### Interactive Lineage Tree

```tsx
// Visual tree showing extraction pipeline
function LineageTree({ lineage }: { lineage: DocumentLineage }) {
  return (
    <div className="space-y-2">
      <LineageNode
        icon={<Upload />}
        label="Document Upload"
        timestamp={lineage.uploaded_at}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<FileSearch />}
        label="Content Extraction"
        details={`${lineage.chunking_strategy} • ${lineage.avg_chunk_size} chars/chunk`}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Brain />}
        label="Entity Extraction"
        details={`${lineage.llm_model} • ${lineage.entity_types?.length} types`}
        duration={lineage.entity_extraction_ms}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Network />}
        label="Relationship Mapping"
        details={`${lineage.relationship_count} relationships found`}
        duration={lineage.relationship_extraction_ms}
        status="completed"
      />
      <LineageConnector />
      <LineageNode
        icon={<Database />}
        label="Graph Indexing"
        details={`${lineage.embedding_model} • ${lineage.embedding_dimensions}D`}
        status="completed"
      />
    </div>
  );
}

function LineageNode({ icon, label, details, duration, status }: LineageNodeProps) {
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg bg-muted/30 hover:bg-muted/50 transition-colors">
      <div className={cn(
        'flex items-center justify-center w-8 h-8 rounded-full',
        status === 'completed' && 'bg-green-500/10',
        status === 'processing' && 'bg-blue-500/10',
        status === 'failed' && 'bg-red-500/10'
      )}>
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between mb-1">
          <span className="text-sm font-medium">{label}</span>
          {duration && (
            <Badge variant="outline" className="text-xs">
              {formatDuration(duration)}
            </Badge>
          )}
        </div>
        {details && (
          <p className="text-xs text-muted-foreground">{details}</p>
        )}
      </div>
    </div>
  );
}
```

## Micro-Interactions & Animations

### Smooth Transitions

```tsx
// Spring-based animations for natural feel
import { useSpring, animated } from '@react-spring/web';

function AnimatedCard({ children, delay = 0 }: AnimatedCardProps) {
  const styles = useSpring({
    from: { opacity: 0, transform: 'translateY(20px)' },
    to: { opacity: 1, transform: 'translateY(0px)' },
    delay,
    config: { tension: 280, friction: 60 }
  });
  
  return (
    <animated.div style={styles}>
      {children}
    </animated.div>
  );
}
```

### Interactive Elements

```tsx
// Hover effects on stat cards
<motion.div
  whileHover={{ scale: 1.02, y: -2 }}
  whileTap={{ scale: 0.98 }}
  transition={{ type: 'spring', stiffness: 400, damping: 17 }}
>
  <StatCard {...props} />
</motion.div>

// Smooth section expansion
<motion.div
  initial={{ height: 0, opacity: 0 }}
  animate={{ height: 'auto', opacity: 1 }}
  exit={{ height: 0, opacity: 0 }}
  transition={{ duration: 0.3, ease: 'easeInOut' }}
>
  {sectionContent}
</motion.div>
```

## Responsive Design

### Breakpoint Strategy

```tsx
// Adaptive layout for all screen sizes
<div className="flex flex-col lg:flex-row h-screen">
  {/* Mobile: Stack vertically with tabs */}
  <Tabs defaultValue="content" className="lg:hidden">
    <TabsList className="w-full">
      <TabsTrigger value="content">Content</TabsTrigger>
      <TabsTrigger value="metadata">Details</TabsTrigger>
    </TabsList>
    <TabsContent value="content">
      <ContentRenderer document={document} />
    </TabsContent>
    <TabsContent value="metadata">
      <MetadataSidebar document={document} />
    </TabsContent>
  </Tabs>
  
  {/* Desktop: Side-by-side */}
  <div className="hidden lg:flex flex-1">
    <ContentRenderer document={document} />
    <MetadataSidebar document={document} />
  </div>
</div>
```

## Performance Optimizations

### 1. **Lazy Loading**
```typescript
// Lazy load heavy renderers
const PDFViewer = lazy(() => import('./pdf-viewer'));
const MermaidDiagram = lazy(() => import('./mermaid-diagram'));
const JSONTreeViewer = lazy(() => import('./json-tree-viewer'));
```

### 2. **Virtualized Lists**
```tsx
// For documents with many entities/relationships
import { useVirtualizer } from '@tanstack/react-virtual';

function EntityList({ entities }: { entities: Entity[] }) {
  const parentRef = useRef<HTMLDivElement>(null);
  
  const virtualizer = useVirtualizer({
    count: entities.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 50,
    overscan: 5,
  });
  
  return (
    <div ref={parentRef} className="h-96 overflow-auto">
      <div style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((virtualRow) => (
          <EntityCard
            key={virtualRow.key}
            entity={entities[virtualRow.index]}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              transform: `translateY(${virtualRow.start}px)`,
            }}
          />
        ))}
      </div>
    </div>
  );
}
```

### 3. **Memoization**
```typescript
// Prevent unnecessary re-renders
const renderedContent = useMemo(
  () => <EnhancedMarkdownRenderer content={document.content} />,
  [document.content]
);

const lineageTree = useMemo(
  () => <LineageTree lineage={document.lineage} />,
  [document.lineage]
);
```

## Implementation Checklist

### Phase 1: Layout & Structure (Days 1-2)
- [ ] Create two-column layout component
- [ ] Implement compact header
- [ ] Build metadata sidebar shell
- [ ] Add responsive breakpoints
- [ ] Create collapsible section component

### Phase 2: Content Renderers (Days 3-4)
- [ ] Enhanced markdown renderer
- [ ] Code syntax highlighter
- [ ] PDF viewer integration
- [ ] Image viewer with lightbox
- [ ] JSON tree viewer
- [ ] Fallback plain text renderer

### Phase 3: Metadata Components (Days 4-5)
- [ ] Key stats cards (sticky)
- [ ] Interactive lineage tree
- [ ] Mini graph preview
- [ ] Entity/relationship cards
- [ ] Processing details section
- [ ] Source info grid

### Phase 4: Interactions & Polish (Days 6-7)
- [ ] Smooth animations (react-spring)
- [ ] Hover effects
- [ ] Copy/download actions
- [ ] Keyboard shortcuts
- [ ] Loading skeletons
- [ ] Error boundaries
- [ ] Mobile optimization

### Phase 5: Performance (Day 7)
- [ ] Lazy loading setup
- [ ] Code splitting
- [ ] Virtualization for long lists
- [ ] Image optimization
- [ ] Bundle size analysis

## Files to Create/Modify

### New Files
```
src/components/document/
├── content-renderer.tsx
├── enhanced-markdown-renderer.tsx
├── code-renderer.tsx
├── pdf-viewer.tsx
├── image-viewer.tsx
├── json-tree-viewer.tsx
├── metadata-sidebar.tsx
├── key-stats.tsx
├── lineage-tree.tsx
├── mini-graph-preview.tsx
├── collapsible-section.tsx
└── document-layout.tsx
```

### Modified Files
```
src/app/(dashboard)/documents/[id]/page.tsx    (Complete rewrite)
src/components/query/markdown-renderer.tsx      (Enhance)
src/types/index.ts                              (Add new types)
```

## Success Criteria

✅ **User Experience**
- Content type automatically detected and rendered appropriately
- Lineage information immediately visible without scrolling
- Smooth, delightful animations (60fps)
- No layout shift during loading
- Mobile-friendly with no horizontal scroll

✅ **Technical**
- Lighthouse score > 90 for all metrics
- Bundle size increase < 50KB (gzipped)
- Time to interactive < 3s
- Proper TypeScript types throughout
- 100% E2E test coverage

✅ **Design**
- Passes design system audit
- WCAG 2.1 AA compliant
- Consistent with rest of application
- Dark mode fully supported

---

**Next:** [Component Library & Design Tokens](./04-component-library.md)
