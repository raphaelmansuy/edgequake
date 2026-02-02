# OODA-67: Breadcrumb Navigation

**Date**: 2026-02-01
**Focus**: Hierarchical Navigation UI

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Clear navigation context
- Easy back navigation

### Current Breadcrumb Implementation

**From document detail page:**
```typescript
<Breadcrumb>
  <BreadcrumbList>
    <BreadcrumbItem>
      <BreadcrumbLink href="/documents">Documents</BreadcrumbLink>
    </BreadcrumbItem>
    <BreadcrumbSeparator />
    <BreadcrumbItem>
      <BreadcrumbPage>{document.title}</BreadcrumbPage>
    </BreadcrumbItem>
  </BreadcrumbList>
</Breadcrumb>
```

## ORIENT

### Breadcrumb Hierarchy

```
Dashboard > Documents > [Document Title]
                            ↑
                     Current page (not linked)
```

### Navigation Context

| Page | Breadcrumb Trail |
|------|------------------|
| Document List | Dashboard > Documents |
| Document Detail | Dashboard > Documents > [Title] |
| Edit Document | Dashboard > Documents > [Title] > Edit |
| Graph View | Dashboard > Knowledge Graph |

## DECIDE

**Decision**: Breadcrumb implementation is correct

The pattern provides:
- Clear location context
- Quick navigation to parents
- Current page identification

## ACT

### Breadcrumb Component

```typescript
interface BreadcrumbItem {
  label: string;
  href?: string;
}

interface BreadcrumbNavProps {
  items: BreadcrumbItem[];
}

const BreadcrumbNav = ({ items }: BreadcrumbNavProps) => (
  <Breadcrumb>
    <BreadcrumbList>
      {items.map((item, index) => (
        <React.Fragment key={index}>
          {index > 0 && <BreadcrumbSeparator />}
          <BreadcrumbItem>
            {item.href ? (
              <BreadcrumbLink href={item.href}>
                {item.label}
              </BreadcrumbLink>
            ) : (
              <BreadcrumbPage>{item.label}</BreadcrumbPage>
            )}
          </BreadcrumbItem>
        </React.Fragment>
      ))}
    </BreadcrumbList>
  </Breadcrumb>
);
```

### Usage Example

```typescript
<BreadcrumbNav items={[
  { label: 'Dashboard', href: '/' },
  { label: 'Documents', href: '/documents' },
  { label: document.title },  // No href = current page
]} />
```

**Status**: ✅ VERIFIED - Breadcrumb navigation complete
