# Markdown Rendering Audit Report

**Date:** December 28, 2025  
**Status:** In Progress  
**Auditor:** EdgeQuake Development Team

---

## Executive Summary

This audit identifies critical bugs in the EdgeQuake markdown rendering system that cause incorrect display of list items with spurious checkboxes. The root cause is a misunderstanding of the `marked.js` ListItem token structure.

---

## Critical Bug #1: Incorrect Task List Detection

### Symptom

All list items display with empty checkboxes, even when the markdown source contains no task list syntax (e.g., `- [ ]` or `- [x]`).

### Screenshot Evidence

![Checkbox Bug](../audit_ui/screenshots/checkbox-bug.png)

The screenshot shows a numbered list with "Products" and nested items like "Code2Doc", "Graphcodebert", etc., all displaying with unwanted checkboxes.

### Root Cause Analysis

**File:** `src/components/query/markdown/MarkdownTokens.tsx` (Lines 190-210)

```tsx
case 'list': {
  const list = token as Tokens.List;
  // ...
  return (
    <Tag className={listStyle} start={list.start || undefined}>
      {list.items.map((item, index) => (
        <li key={index} className="leading-7">
          {item.task !== undefined && (   // ❌ BUG: Always true!
            <input
              type="checkbox"
              checked={item.checked}
              disabled
              className="mr-2 h-4 w-4 rounded border-zinc-600"
            />
          )}
          ...
        </li>
      ))}
    </Tag>
  );
}
```

**The Bug:** The condition `item.task !== undefined` is ALWAYS true.

According to marked.js TypeScript definitions:

```typescript
interface ListItem {
  type: "list_item";
  raw: string;
  task: boolean; // ← Always defined, either true or false
  checked?: boolean; // ← Optional, only present when task=true
  loose: boolean;
  text: string;
  tokens: Token[];
}
```

The `task` property is a required `boolean` field (not optional), so it's always defined as either `true` or `false`. The check should be `item.task === true`, not `item.task !== undefined`.

### Correct Implementation (from open-webui)

**File:** `open-webui/src/lib/components/chat/Messages/Markdown/MarkdownTokens.svelte` (Lines 268-296)

```svelte
{:else if token.type === 'list'}
  {#if token.ordered}
    <ol start={token.start || 1} dir="auto">
      {#each token.items as item, itemIdx}
        <li class="text-start">
          {#if item?.task}  <!-- ✅ Correct: Checks if task is truthy -->
            <input
              type="checkbox"
              checked={item.checked}
              on:change={(e) => { ... }}
            />
          {/if}
          ...
        </li>
      {/each}
    </ol>
  {:else}
    <ul dir="auto">
      {#each token.items as item, itemIdx}
        <li class="text-start {item?.task ? 'flex -translate-x-6.5 gap-3' : ''}">
          {#if item?.task}  <!-- ✅ Correct: Checks if task is truthy -->
            <input type="checkbox" checked={item.checked} ... />
            <div>...</div>
          {:else}
            ...
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
```

### Fix Required

```tsx
// BEFORE (buggy)
{item.task !== undefined && (
  <input type="checkbox" ... />
)}

// AFTER (correct)
{item.task && (
  <input type="checkbox" ... />
)}
```

---

## Additional Issues Identified

### Issue #2: Missing Task List Styling

When a list IS a task list, the styling doesn't match open-webui's polished appearance:

- No horizontal alignment adjustment
- No interactive checkbox (currently disabled)
- Missing flex layout for proper alignment

### Issue #3: List Structure Not Handling Loose vs Tight

The `loose` property on list items affects paragraph wrapping:

- `loose: true` → List items should wrap content in `<p>` tags
- `loose: false` → List items should render content inline

Current implementation ignores this distinction.

### Issue #4: Missing Nested List Proper Indentation

Nested lists don't properly inherit or increase indentation levels, causing visual confusion.

---

## Comparison with open-webui

| Feature                | EdgeQuake                 | open-webui              | Status               |
| ---------------------- | ------------------------- | ----------------------- | -------------------- |
| Task list detection    | `item.task !== undefined` | `item?.task`            | ❌ Bug               |
| Interactive checkboxes | No (disabled)             | Yes (with callback)     | ⚠️ Missing feature   |
| Loose list handling    | Ignored                   | `top={token.loose}`     | ⚠️ Missing           |
| CSS for task items     | Basic                     | Flex layout with offset | ⚠️ Needs improvement |
| Ordered list start     | ✅ Implemented            | ✅ Implemented          | ✅ OK                |
| Unordered list styling | ✅ Basic                  | ✅ Enhanced             | ⚠️ Needs improvement |

---

## open-webui Architecture Analysis

### Key Differences

1. **Extension System**

   - open-webui uses separate extension files in `src/lib/utils/marked/`:

     - `extension.ts` - Details/collapsible blocks
     - `katex-extension.ts` - Math rendering
     - `citation-extension.ts` - Source citations
     - `footnote-extension.ts` - Footnotes
     - `mention-extension.ts` - @mentions and #tags
     - `strikethrough-extension.ts` - Strikethrough customization

   - EdgeQuake bundles everything in `configure-marked.ts`

2. **Component Structure**

   - open-webui: Svelte with recursive `<svelte:self>` for nested structures
   - EdgeQuake: React with recursive component calls

3. **Alert Rendering**

   - open-webui: Detects alerts inside blockquotes using `alertComponent(token)`
   - EdgeQuake: Uses custom tokenizer for `github_alert` type

4. **HTML Token Handling**
   - open-webui: Dedicated `HtmlToken.svelte` component
   - EdgeQuake: Inline with DOMPurify sanitization

---

## Recommended Fixes

### Priority 1: Critical (Fix Now)

1. **Fix task list detection** - Change `item.task !== undefined` to `item.task`

### Priority 2: High (Sprint 1)

2. **Add loose list handling** - Respect the `loose` property for paragraph wrapping
3. **Improve task list styling** - Add flex layout and proper alignment
4. **Add interactive checkboxes** - Optional callback for task toggling

### Priority 3: Medium (Sprint 2)

5. **Refactor extensions** - Split into separate files like open-webui
6. **Enhance nested list indentation** - Better visual hierarchy
7. **Add comprehensive test suite** - Unit + E2E tests

---

## Files Requiring Changes

| File                               | Change Type           | Priority |
| ---------------------------------- | --------------------- | -------- |
| `MarkdownTokens.tsx`               | Bug fix + Enhancement | P1+P2    |
| `StreamingMarkdownRenderer.tsx`    | Verification          | P2       |
| `globals.css`                      | Styling additions     | P2       |
| New: `__tests__/markdown.test.tsx` | New file              | P3       |
| New: `e2e/markdown.spec.ts`        | New file              | P3       |

---

## Next Steps

1. Create testing methodology document
2. Create implementation plan with test cases
3. Implement fixes in priority order
4. Verify with automated tests
