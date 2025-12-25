# UI Audit: Edit Entity Dialog

**Screen:** Edit Entity Modal/Dialog  
**Date:** 2025-12-25  
**Priority:** High - Core CRUD functionality

---

## Screenshot Analysis

Modal dialog for editing entity properties:
- Dialog title with edit icon
- Entity Name input (text field)
- Entity Type dropdown (select)
- Description textarea
- Properties section (read-only display)
- Cancel and Save buttons

---

## Issues Identified

### Critical Issues

| ID | Issue | Location | Severity |
|----|-------|----------|----------|
| EED-01 | **Properties are read-only but not obviously so** - User might expect to edit all properties but they're display-only | Properties section | 🔴 Critical |
| EED-02 | **Description label duplicated in Properties** - "description" shows in both textarea and properties list with same value | Description / Properties | 🔴 Critical |

### High Priority Issues

| ID | Issue | Location | Severity |
|----|-------|----------|----------|
| EED-03 | **No form validation indicators** - No asterisks for required fields, no character limits shown | Form fields | 🟠 High |
| EED-04 | **Cancel/Save buttons small** - Action buttons appear compact with minimal padding | Footer | 🟠 High |
| EED-05 | **Property values truncated** - UUIDs like `c52657ee-fd49-4737-89de-6f473786edea` cut off | Properties | 🟠 High |
| EED-06 | **No dirty state warning** - If user has changes and clicks Cancel, no confirmation | Cancel behavior | 🟠 High |

### Medium Priority Issues

| ID | Issue | Location | Severity |
|----|-------|----------|----------|
| EED-07 | **Dialog subtitle wraps awkwardly** - "Modify the entity properties. Renaming may trigger a merge if another entity with the same name exists." | Header | 🟡 Medium |
| EED-08 | **Entity Type dropdown small** - Compact width, could show more types at once | Type selector | 🟡 Medium |
| EED-09 | **Properties section styling** - Inconsistent with form field styling above | Properties section | 🟡 Medium |
| EED-10 | **No loading state** - Save button doesn't show loading during API call | Save button | 🟡 Medium |
| EED-11 | **Entity Name pre-selected** - "Qwen3-30B" is selected on open, user might accidentally clear it | Name input | 🟡 Medium |

### Low Priority Issues

| ID | Issue | Location | Severity |
|----|-------|----------|----------|
| EED-12 | **Sparkle icon in Properties** - "✦ Properties" uses decorative icon, may be unnecessary | Section header | 🟢 Low |
| EED-13 | **Importance shown as number** - 0.5 could be a slider or visual indicator | Properties | 🟢 Low |
| EED-14 | **Modal backdrop opacity** - Gray overlay could be darker for better focus | Backdrop | 🟢 Low |

---

## Improvement Plan

### Phase 1: Critical Fixes (Week 1)

#### 1.1 Separate Editable vs Read-Only Fields
```
Current:
┌─────────────────────────────────────────────────┐
│ Entity Name      [Qwen3-30B____________]        │
│ Entity Type      [PRODUCT ▼]                    │
│ Description      [A benchmark model...]         │
│                                                 │
│ ✦ Properties                                    │
│ description      A benchmark model...     ← DUP │
│ entity_type      PRODUCT                  ← DUP │
│ importance       0.5                            │
│ source_ids       c52657ee-fd49-...              │
└─────────────────────────────────────────────────┘

Proposed:
┌─────────────────────────────────────────────────┐
│ ✎ EDITABLE PROPERTIES                           │
├─────────────────────────────────────────────────┤
│ Entity Name *    [Qwen3-30B____________]        │
│ Entity Type      [PRODUCT ▼]                    │
│ Description      [A benchmark model...]         │
│                  0/500 characters               │
├─────────────────────────────────────────────────┤
│ 🔒 SYSTEM PROPERTIES (read-only)                │
├─────────────────────────────────────────────────┤
│ importance       0.5          [████░░░░░░]      │
│ source_ids       c52657ee-... [📋 Copy]         │
│ tenant_id        7f8d921e-... [📋 Copy]         │
│ workspace_id     d3d520d1-... [📋 Copy]         │
└─────────────────────────────────────────────────┘
```

#### 1.2 Remove Duplicate Properties
- Remove `description` and `entity_type` from Properties section
- These are already shown in editable form fields
- Keep only system-managed properties

### Phase 2: Form Validation (Week 1)

#### 2.1 Required Field Indicators
```tsx
<Label htmlFor="entityName">
  Entity Name <span className="text-destructive">*</span>
</Label>
<Input 
  id="entityName"
  value={name}
  onChange={(e) => setName(e.target.value)}
  required
  aria-required="true"
/>
{errors.name && (
  <p className="text-sm text-destructive mt-1">{errors.name}</p>
)}
```

#### 2.2 Character Count
```tsx
<div className="space-y-2">
  <Label htmlFor="description">Description</Label>
  <Textarea
    id="description"
    value={description}
    onChange={(e) => setDescription(e.target.value)}
    maxLength={500}
    className="min-h-[100px]"
  />
  <p className="text-xs text-muted-foreground text-right">
    {description.length}/500 characters
  </p>
</div>
```

#### 2.3 Validation Rules
```typescript
const validationSchema = {
  name: {
    required: true,
    minLength: 1,
    maxLength: 100,
    pattern: /^[a-zA-Z0-9\s\-_]+$/,
    message: "Name must contain only letters, numbers, spaces, hyphens, or underscores"
  },
  description: {
    maxLength: 500
  }
};
```

### Phase 3: Dirty State & Confirmation (Week 1)

#### 3.1 Unsaved Changes Warning
```tsx
const [hasChanges, setHasChanges] = useState(false);

const handleCancel = () => {
  if (hasChanges) {
    setShowConfirmDialog(true);
  } else {
    onClose();
  }
};

// Confirmation dialog
<AlertDialog open={showConfirmDialog}>
  <AlertDialogContent>
    <AlertDialogHeader>
      <AlertDialogTitle>Discard changes?</AlertDialogTitle>
      <AlertDialogDescription>
        You have unsaved changes. Are you sure you want to close without saving?
      </AlertDialogDescription>
    </AlertDialogHeader>
    <AlertDialogFooter>
      <AlertDialogCancel onClick={() => setShowConfirmDialog(false)}>
        Keep editing
      </AlertDialogCancel>
      <AlertDialogAction onClick={onClose}>
        Discard
      </AlertDialogAction>
    </AlertDialogFooter>
  </AlertDialogContent>
</AlertDialog>
```

### Phase 4: Button & Loading States (Week 2)

#### 4.1 Improved Button Layout
```
Current:
         [Cancel] [Save]

Proposed:
┌─────────────────────────────────────────────────┐
│                                                 │
│  [Cancel]                        [💾 Save]      │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Specifications:**
- Cancel: `variant="outline"` aligned left
- Save: `variant="default"` aligned right, with icon
- Button height: `h-10` (40px)
- Gap between buttons: `gap-3`

#### 4.2 Loading State
```tsx
<Button 
  type="submit" 
  disabled={isSaving || !hasChanges}
  className="min-w-[100px]"
>
  {isSaving ? (
    <>
      <Loader2 className="h-4 w-4 animate-spin mr-2" />
      Saving...
    </>
  ) : (
    <>
      <Save className="h-4 w-4 mr-2" />
      Save
    </>
  )}
</Button>
```

### Phase 5: UI Polish (Week 2)

#### 5.1 Dialog Header Improvements
```
Current:
✎ Edit Entity
Modify the entity properties. Renaming may trigger a merge if 
another entity with the same name exists.

Proposed:
┌─────────────────────────────────────────────────┐
│ ✎ Edit Entity                              [×] │
├─────────────────────────────────────────────────┤
│ Update the properties for this entity.          │
│                                                 │
│ ⚠️ Note: Renaming may merge with an existing   │
│    entity if names match.                       │
└─────────────────────────────────────────────────┘
```

#### 5.2 Entity Type Dropdown Enhancement
```tsx
<Select value={entityType} onValueChange={setEntityType}>
  <SelectTrigger className="w-[180px]">
    <SelectValue>
      <div className="flex items-center gap-2">
        <span className={cn(
          "h-2 w-2 rounded-full",
          typeColors[entityType]
        )} />
        {entityType}
      </div>
    </SelectValue>
  </SelectTrigger>
  <SelectContent>
    {entityTypes.map(type => (
      <SelectItem key={type} value={type}>
        <div className="flex items-center gap-2">
          <span className={cn(
            "h-2 w-2 rounded-full",
            typeColors[type]
          )} />
          {type}
        </div>
      </SelectItem>
    ))}
  </SelectContent>
</Select>
```

---

## Proposed Dialog Component

```tsx
function EditEntityDialog({ entity, open, onOpenChange, onSave }) {
  const [formData, setFormData] = useState({
    name: entity.name,
    type: entity.type,
    description: entity.description
  });
  const [isSaving, setIsSaving] = useState(false);
  const [showDiscardDialog, setShowDiscardDialog] = useState(false);
  
  const hasChanges = useMemo(() => {
    return formData.name !== entity.name ||
           formData.type !== entity.type ||
           formData.description !== entity.description;
  }, [formData, entity]);

  const handleClose = () => {
    if (hasChanges) {
      setShowDiscardDialog(true);
    } else {
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Edit className="h-5 w-5" />
            Edit Entity
          </DialogTitle>
          <DialogDescription>
            Update the properties for this entity.
          </DialogDescription>
        </DialogHeader>
        
        {/* Merge Warning */}
        <Alert variant="warning" className="mt-2">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription className="text-xs">
            Renaming may merge with an existing entity if names match.
          </AlertDescription>
        </Alert>
        
        <form onSubmit={handleSubmit} className="space-y-6 mt-4">
          {/* Editable Fields */}
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">
                Entity Name <span className="text-destructive">*</span>
              </Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) => setFormData({...formData, name: e.target.value})}
                required
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="type">Entity Type</Label>
              <EntityTypeSelect
                value={formData.type}
                onChange={(type) => setFormData({...formData, type})}
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={formData.description}
                onChange={(e) => setFormData({...formData, description: e.target.value})}
                maxLength={500}
                className="min-h-[100px]"
              />
              <p className="text-xs text-muted-foreground text-right">
                {formData.description.length}/500
              </p>
            </div>
          </div>
          
          {/* System Properties */}
          <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Lock className="h-4 w-4" />
              System Properties
            </div>
            <div className="rounded-lg border bg-muted/30 p-4 space-y-2 text-sm">
              <PropertyRow label="importance" value={entity.importance} />
              <PropertyRow label="source_ids" value={entity.source_ids} copyable />
              <PropertyRow label="tenant_id" value={entity.tenant_id} copyable />
              <PropertyRow label="workspace_id" value={entity.workspace_id} copyable />
            </div>
          </div>
        </form>
        
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={handleClose}>
            Cancel
          </Button>
          <Button 
            onClick={handleSubmit}
            disabled={isSaving || !hasChanges}
          >
            {isSaving ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
                Saving...
              </>
            ) : (
              'Save'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
      
      {/* Discard Confirmation */}
      <DiscardChangesDialog
        open={showDiscardDialog}
        onConfirm={() => onOpenChange(false)}
        onCancel={() => setShowDiscardDialog(false)}
      />
    </Dialog>
  );
}
```

---

## Accessibility Improvements

1. **Form Accessibility:**
   - All inputs have associated labels
   - Required fields marked with `aria-required="true"`
   - Error messages linked with `aria-describedby`

2. **Keyboard Navigation:**
   - Tab through form fields
   - Enter to submit
   - Escape to close (with confirmation if dirty)

3. **Screen Reader:**
   - "Edit Entity dialog, Entity Name, required, Qwen3-30B"
   - Announce validation errors
   - Announce save success/failure

---

## Success Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Read-only field clarity | Ambiguous | Clear "System Properties" section |
| Form validation | None visible | Required indicators + errors |
| Dirty state handling | None | Confirmation dialog |
| Save feedback | None | Loading state + success toast |
