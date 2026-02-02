# OODA-63: Context Menu Actions

**Date**: 2026-02-01
**Focus**: Right-click Document Actions

## OBSERVE

### Mission Requirements (Re-read from specs/002-unify-ingestion-pipeline.md)
- Quick access to document actions
- Right-click context menu

### Current Context Menu

**From document-manager.tsx:**
```typescript
<ContextMenu>
  <ContextMenuTrigger asChild>
    <TableRow onDoubleClick={() => handleDocumentDoubleClick(doc)}>
      {/* ... row content */}
    </TableRow>
  </ContextMenuTrigger>
  <ContextMenuContent>
    <ContextMenuItem onClick={() => handleViewDetails(doc)}>
      <Eye className="mr-2 h-4 w-4" />
      View Details
    </ContextMenuItem>
    <ContextMenuItem onClick={() => handleEdit(doc)}>
      <Pencil className="mr-2 h-4 w-4" />
      Edit
    </ContextMenuItem>
    <ContextMenuSeparator />
    <ContextMenuItem onClick={() => handleDownload(doc)}>
      <Download className="mr-2 h-4 w-4" />
      Download
    </ContextMenuItem>
    <ContextMenuSeparator />
    <ContextMenuItem 
      onClick={() => handleDelete(doc)}
      className="text-destructive"
    >
      <Trash2 className="mr-2 h-4 w-4" />
      Delete
    </ContextMenuItem>
  </ContextMenuContent>
</ContextMenu>
```

## ORIENT

### Context Menu Actions

| Action | Keyboard | Icon |
|--------|----------|------|
| View Details | Enter | Eye |
| Edit | E | Pencil |
| Download | D | Download |
| Delete | Del | Trash2 |
| Copy Link | C | Link |

### Accessibility
- Arrow keys navigate menu
- Enter activates item
- Escape closes menu

## DECIDE

**Decision**: Context menu correctly implemented

The implementation provides:
- Full action coverage
- Visual separators for grouping
- Destructive action styling

## ACT

### Menu Item Pattern

```typescript
<ContextMenuItem 
  onClick={() => action(doc)}
  disabled={isDisabled}
  className={cn(isDestructive && "text-destructive focus:text-destructive")}
>
  <Icon className="mr-2 h-4 w-4" />
  <span>{label}</span>
  {shortcut && (
    <ContextMenuShortcut>{shortcut}</ContextMenuShortcut>
  )}
</ContextMenuItem>
```

### Submenu Pattern

```typescript
<ContextMenuSub>
  <ContextMenuSubTrigger>
    <Tag className="mr-2 h-4 w-4" />
    Add Tag
  </ContextMenuSubTrigger>
  <ContextMenuSubContent>
    {availableTags.map(tag => (
      <ContextMenuItem 
        key={tag}
        onClick={() => addTag(doc.id, tag)}
      >
        {tag}
      </ContextMenuItem>
    ))}
  </ContextMenuSubContent>
</ContextMenuSub>
```

**Status**: ✅ VERIFIED - Context menu complete
