# Plan: Workspace Fuzzy Search Selector

**Addresses:** F-WS-01, F-WS-03, F-WS-04, F-WS-05  
**Component to replace:** `header-tenant-selector.tsx`  
**Pattern:** `Popover` + `Command` (cmdk) — same pattern used in shadcn combobox  
**Reference:** [shadcn/ui Combobox](https://ui.shadcn.com/docs/components/combobox)

---

## Design Spec

```
┌─ Header ─────────────────────────────────────────────────────────┐
│  [ ≡ Default  /  Default Workspace ▾ ]                           │
└──────────────────────────────────────────────────────────────────┘
         │ click/keyboard
         ▼
┌─ Popover (w-72) ──────────────────────────────────────────────────┐
│  ┌─ Command ─────────────────────────────────────────────────────┐ │
│  │  🔍 Search workspaces...                                       │ │
│  │  ──────────────────────────────────────────────────────────── │ │
│  │  Organizations                                                 │ │
│  │  ├ ✓ Default                                           (curr) │ │
│  │  └   Production                                               │ │
│  │  ──────────────────────────────────────────────────────────── │ │
│  │  Workspaces — Default                                          │ │
│  │  ├ ✓ Default Workspace                                (curr) │ │
│  │  └   Research                                                 │ │
│  │  ──────────────────────────────────────────────────────────── │ │
│  │  + New Workspace                                               │ │
│  │  + New Organization                                            │ │
│  └───────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Steps

1. Replace `DropdownMenu` with `Popover` + `Command` components
2. Add `CommandInput` for fuzzy search (cmdk handles filtering automatically)
3. Group by `CommandGroup`: Organizations → Workspaces (scoped to selected org)
4. Show check mark on currently selected tenant/workspace
5. Keep create dialogs accessible from the bottom of the Command list
6. Preserve all existing mutation/query logic unchanged

---

## Key Code Change (DRY — no logic duplication)

```tsx
// header-tenant-selector.tsx
// Replace:
<DropdownMenu>
  <DropdownMenuTrigger>...</DropdownMenuTrigger>
  <DropdownMenuContent>
    {tenants.map(...)}
  </DropdownMenuContent>
</DropdownMenu>

// With:
<Popover open={open} onOpenChange={setOpen}>
  <PopoverTrigger asChild>
    <Button variant="ghost" ...>
      ...trigger content...
    </Button>
  </PopoverTrigger>
  <PopoverContent className="w-72 p-0">
    <Command>
      <CommandInput placeholder="Search workspaces..." />
      <CommandList>
        <CommandGroup heading="Organizations">
          {tenants.map(tenant => (
            <CommandItem
              key={tenant.id}
              value={tenant.name}
              onSelect={() => handleTenantSelect(tenant.id)}
            >
              <Check className={cn("mr-2 h-4 w-4", tenant.id === selectedTenantId ? "opacity-100" : "opacity-0")} />
              {tenant.name}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup heading={`Workspaces — ${selectedTenant?.name}`}>
          {workspaces.map(ws => (
            <CommandItem key={ws.id} value={ws.name} onSelect={() => handleWorkspaceSelect(ws.id)}>
              <Check className={cn("mr-2 h-4 w-4", ws.id === selectedWorkspaceId ? "opacity-100" : "opacity-0")} />
              {ws.name}
            </CommandItem>
          ))}
        </CommandGroup>
        <CommandSeparator />
        <CommandGroup>
          <CommandItem onSelect={() => { setOpen(false); setShowCreateWorkspace(true); }}>
            <Plus className="mr-2 h-4 w-4" /> New Workspace
          </CommandItem>
          <CommandItem onSelect={() => { setOpen(false); setShowCreateTenant(true); }}>
            <Plus className="mr-2 h-4 w-4" /> New Organization
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </Command>
  </PopoverContent>
</Popover>
```

---

## Acceptance Criteria

- [ ] Typing in the search box filters both tenants and workspaces
- [ ] Arrow keys navigate the list
- [ ] Enter selects the focused item
- [ ] Escape closes the popover
- [ ] Currently selected tenant/workspace shows a check mark
- [ ] With 1 tenant and 1 workspace (default state), the dropdown is not confusingly sparse
- [ ] Existing create-tenant and create-workspace dialogs still work
