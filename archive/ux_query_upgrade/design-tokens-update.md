# Design Tokens Update Specification

> New and updated CSS custom properties for the chat interface UX upgrade.

## Overview

This document specifies the design tokens (CSS custom properties) that need to be added or updated in `src/app/design-tokens.css` to support the enhanced query interface.

---

## New Token Categories

### Chat Message Tokens

```css
:root {
  /* === CHAT MESSAGE SYSTEM === */
  
  /* Message container constraints */
  --chat-message-max-width: 800px;
  --chat-message-gap: var(--spacing-4);
  
  /* User message bubble */
  --chat-user-bg: oklch(0.95 0.02 265);
  --chat-user-bg-dark: oklch(0.25 0.02 265);
  --chat-user-text: var(--foreground);
  --chat-user-radius: var(--radius-lg);
  
  /* Assistant message bubble */
  --chat-assistant-bg: transparent;
  --chat-assistant-bg-dark: transparent;
  --chat-assistant-text: var(--foreground);
  --chat-assistant-radius: var(--radius-lg);
  --chat-assistant-border: oklch(0.9 0.01 265);
  --chat-assistant-border-dark: oklch(0.3 0.01 265);
  
  /* Message shadow (depth) */
  --chat-message-shadow: 0 1px 3px oklch(0 0 0 / 0.08);
  --chat-message-shadow-dark: 0 1px 3px oklch(0 0 0 / 0.2);
  --chat-message-shadow-hover: 0 2px 6px oklch(0 0 0 / 0.12);
  --chat-message-shadow-hover-dark: 0 2px 6px oklch(0 0 0 / 0.3);
  
  /* Avatar */
  --chat-avatar-size: 32px;
  --chat-avatar-bg: var(--primary);
  --chat-avatar-text: var(--primary-foreground);
  
  /* Metadata footer */
  --chat-metadata-text: var(--muted-foreground);
  --chat-metadata-font-size: var(--text-xs);
  
  /* Thinking/reasoning section */
  --chat-thinking-bg: oklch(0.97 0.01 280);
  --chat-thinking-bg-dark: oklch(0.18 0.01 280);
  --chat-thinking-border: oklch(0.9 0.02 280);
  --chat-thinking-border-dark: oklch(0.3 0.02 280);
  --chat-thinking-text: var(--muted-foreground);
}
```

### Code Block Tokens

```css
:root {
  /* === CODE BLOCK SYSTEM === */
  
  /* Container */
  --code-block-bg: oklch(0.15 0.01 265);
  --code-block-bg-dark: oklch(0.12 0.01 265);
  --code-block-radius: var(--radius-lg);
  --code-block-border: oklch(0.25 0.01 265);
  --code-block-border-dark: oklch(0.2 0.01 265);
  
  /* Header bar */
  --code-header-bg: oklch(0.2 0.01 265);
  --code-header-bg-dark: oklch(0.16 0.01 265);
  --code-header-text: oklch(0.7 0.01 265);
  --code-header-height: 40px;
  
  /* Line numbers */
  --code-line-number-width: 48px;
  --code-line-number-bg: oklch(0.12 0.01 265);
  --code-line-number-color: oklch(0.45 0.01 265);
  --code-line-number-border: oklch(0.2 0.01 265);
  
  /* Code text */
  --code-font-family: 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace;
  --code-font-size: 0.875rem;
  --code-line-height: 1.6;
  --code-text-color: oklch(0.9 0.01 265);
  
  /* Syntax highlighting (base colors) */
  --code-keyword: oklch(0.75 0.15 280);
  --code-string: oklch(0.72 0.12 160);
  --code-number: oklch(0.75 0.12 60);
  --code-function: oklch(0.78 0.12 210);
  --code-comment: oklch(0.5 0.02 265);
  --code-operator: oklch(0.85 0.08 40);
  --code-type: oklch(0.75 0.12 180);
  --code-variable: oklch(0.85 0.05 265);
  
  /* Copy button */
  --code-copy-bg: oklch(0.25 0.01 265);
  --code-copy-bg-hover: oklch(0.35 0.01 265);
  --code-copy-text: oklch(0.7 0.01 265);
  
  /* Inline code */
  --code-inline-bg: oklch(0.94 0.01 265);
  --code-inline-bg-dark: oklch(0.22 0.01 265);
  --code-inline-padding: 0.125em 0.375em;
  --code-inline-radius: var(--radius-sm);
}
```

### Mermaid Diagram Tokens

```css
:root {
  /* === MERMAID DIAGRAM SYSTEM === */
  
  /* Container */
  --mermaid-container-bg: var(--card);
  --mermaid-container-border: var(--border);
  --mermaid-container-radius: var(--radius-lg);
  --mermaid-container-padding: var(--spacing-4);
  
  /* Header */
  --mermaid-header-height: 44px;
  --mermaid-header-border: var(--border);
  
  /* Controls */
  --mermaid-control-bg: var(--secondary);
  --mermaid-control-bg-hover: var(--secondary-foreground);
  --mermaid-control-size: 32px;
  
  /* Zoom */
  --mermaid-zoom-min: 0.5;
  --mermaid-zoom-max: 3;
  --mermaid-zoom-step: 0.25;
  
  /* Error state */
  --mermaid-error-bg: oklch(0.97 0.03 25);
  --mermaid-error-bg-dark: oklch(0.2 0.03 25);
  --mermaid-error-border: oklch(0.8 0.1 25);
  --mermaid-error-text: oklch(0.5 0.15 25);
}
```

### Input Area Tokens

```css
:root {
  /* === INPUT AREA SYSTEM === */
  
  /* Container */
  --input-area-bg: var(--background);
  --input-area-bg-blur: oklch(var(--background) / 0.85);
  --input-area-border: var(--border);
  --input-area-padding: var(--spacing-4);
  
  /* Textarea */
  --input-textarea-bg: var(--card);
  --input-textarea-border: var(--border);
  --input-textarea-border-focus: var(--primary);
  --input-textarea-radius: var(--radius-lg);
  --input-textarea-min-height: 48px;
  --input-textarea-max-height: 200px;
  
  /* Ring (focus) */
  --input-ring-width: 2px;
  --input-ring-color: var(--ring);
  --input-ring-offset: 2px;
  
  /* Send button */
  --input-send-size: 40px;
  --input-send-bg: var(--primary);
  --input-send-bg-disabled: var(--muted);
  --input-send-text: var(--primary-foreground);
  --input-send-radius: var(--radius-md);
  
  /* Mode selector */
  --input-mode-bg: var(--secondary);
  --input-mode-bg-active: var(--primary);
  --input-mode-text: var(--secondary-foreground);
  --input-mode-text-active: var(--primary-foreground);
  --input-mode-radius: var(--radius-full);
  
  /* Character count */
  --input-char-count-text: var(--muted-foreground);
  --input-char-count-warning: oklch(0.7 0.15 60);
  --input-char-count-error: oklch(0.65 0.2 25);
}
```

### Animation Tokens

```css
:root {
  /* === ANIMATION SYSTEM === */
  
  /* Durations */
  --duration-instant: 50ms;
  --duration-fast: 100ms;
  --duration-normal: 200ms;
  --duration-slow: 300ms;
  --duration-slower: 500ms;
  
  /* Easings */
  --ease-in: cubic-bezier(0.4, 0, 1, 1);
  --ease-out: cubic-bezier(0, 0, 0.2, 1);
  --ease-in-out: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-bounce: cubic-bezier(0.34, 1.56, 0.64, 1);
  --ease-spring: cubic-bezier(0.22, 0.68, 0, 1.71);
  
  /* Message animations */
  --animation-message-slide: 200ms var(--ease-out);
  --animation-message-fade: 150ms var(--ease-out);
  
  /* Panel animations */
  --animation-panel-slide: 200ms var(--ease-out);
  --animation-panel-fade: 150ms var(--ease-out);
  
  /* Micro-interactions */
  --animation-button-press: 100ms var(--ease-out);
  --animation-hover-lift: 150ms var(--ease-out);
  
  /* Loading animations */
  --animation-shimmer-duration: 1.5s;
  --animation-pulse-duration: 1.5s;
  --animation-dot-bounce-duration: 1.4s;
}
```

### Panel & Layout Tokens

```css
:root {
  /* === PANEL SYSTEM === */
  
  /* History panel */
  --history-panel-width: 280px;
  --history-panel-width-collapsed: 0px;
  --history-panel-bg: var(--card);
  --history-panel-border: var(--border);
  --history-panel-header-height: 56px;
  --history-panel-search-height: 40px;
  
  /* Details panel */
  --details-panel-width: 320px;
  --details-panel-bg: var(--card);
  --details-panel-border: var(--border);
  
  /* Chat area */
  --chat-area-max-width: 800px;
  --chat-area-padding: var(--spacing-6);
  --chat-area-padding-mobile: var(--spacing-4);
  
  /* Mobile bottom sheet */
  --bottom-sheet-handle-height: 20px;
  --bottom-sheet-handle-width: 36px;
  --bottom-sheet-max-height: 70vh;
  --bottom-sheet-radius: var(--radius-xl);
}
```

---

## Dark Mode Variants

All tokens should have dark mode equivalents. Use the `.dark` selector pattern:

```css
.dark {
  /* Chat Message */
  --chat-user-bg: var(--chat-user-bg-dark);
  --chat-message-shadow: var(--chat-message-shadow-dark);
  --chat-assistant-border: var(--chat-assistant-border-dark);
  --chat-thinking-bg: var(--chat-thinking-bg-dark);
  --chat-thinking-border: var(--chat-thinking-border-dark);
  
  /* Code Block */
  --code-block-bg: var(--code-block-bg-dark);
  --code-block-border: var(--code-block-border-dark);
  --code-header-bg: var(--code-header-bg-dark);
  --code-inline-bg: var(--code-inline-bg-dark);
  
  /* Mermaid */
  --mermaid-error-bg: var(--mermaid-error-bg-dark);
}
```

---

## Semantic Token Mappings

Map new tokens to existing design system tokens where possible:

```css
:root {
  /* Use existing spacing scale */
  --spacing-px: 1px;
  --spacing-0: 0;
  --spacing-0-5: 0.125rem;   /* 2px */
  --spacing-1: 0.25rem;      /* 4px */
  --spacing-1-5: 0.375rem;   /* 6px */
  --spacing-2: 0.5rem;       /* 8px */
  --spacing-2-5: 0.625rem;   /* 10px */
  --spacing-3: 0.75rem;      /* 12px */
  --spacing-3-5: 0.875rem;   /* 14px */
  --spacing-4: 1rem;         /* 16px */
  --spacing-5: 1.25rem;      /* 20px */
  --spacing-6: 1.5rem;       /* 24px */
  --spacing-7: 1.75rem;      /* 28px */
  --spacing-8: 2rem;         /* 32px */
  --spacing-9: 2.25rem;      /* 36px */
  --spacing-10: 2.5rem;      /* 40px */
  --spacing-11: 2.75rem;     /* 44px */
  --spacing-12: 3rem;        /* 48px */
  --spacing-14: 3.5rem;      /* 56px */
  --spacing-16: 4rem;        /* 64px */
  
  /* Use existing radius scale */
  --radius-none: 0;
  --radius-sm: 0.25rem;      /* 4px */
  --radius-md: 0.375rem;     /* 6px */
  --radius-lg: 0.5rem;       /* 8px */
  --radius-xl: 0.75rem;      /* 12px */
  --radius-2xl: 1rem;        /* 16px */
  --radius-3xl: 1.5rem;      /* 24px */
  --radius-full: 9999px;
  
  /* Use existing typography scale */
  --text-xs: 0.75rem;        /* 12px */
  --text-sm: 0.875rem;       /* 14px */
  --text-base: 1rem;         /* 16px */
  --text-lg: 1.125rem;       /* 18px */
  --text-xl: 1.25rem;        /* 20px */
  --text-2xl: 1.5rem;        /* 24px */
}
```

---

## Usage Guidelines

### Accessing Tokens in CSS

```css
.chat-message {
  max-width: var(--chat-message-max-width);
  box-shadow: var(--chat-message-shadow);
  animation: slide-in var(--animation-message-slide);
}

.chat-message:hover {
  box-shadow: var(--chat-message-shadow-hover);
}
```

### Accessing Tokens in Tailwind

Configure `tailwind.config.ts` to expose tokens:

```typescript
export default {
  theme: {
    extend: {
      colors: {
        'chat-user': 'var(--chat-user-bg)',
        'chat-assistant': 'var(--chat-assistant-bg)',
        'code-block': 'var(--code-block-bg)',
      },
      maxWidth: {
        'chat': 'var(--chat-message-max-width)',
      },
      animation: {
        'message-slide': 'slide-in var(--animation-message-slide)',
      },
    },
  },
}
```

### Accessing Tokens in JavaScript

```typescript
const style = getComputedStyle(document.documentElement);
const maxWidth = style.getPropertyValue('--chat-message-max-width');
```

---

## Migration Notes

### Existing Tokens to Keep

The following tokens from `design-tokens.css` remain unchanged:
- All color primitives (`--background`, `--foreground`, `--primary`, etc.)
- Spacing scale (`--spacing-*`)
- Typography scale (`--text-*`)
- Radius scale (`--radius-*`)
- Panel widths (`--panel-*`)

### Tokens to Update

```css
/* Before */
--panel-left-width: 250px;

/* After */
--panel-left-width: var(--history-panel-width);
```

### Tokens to Deprecate

The following tokens can be deprecated after migration:
- Any hardcoded pixel values in component-level CSS
- Inline color values in component files
- Duplicated spacing values

---

## Implementation Checklist

- [ ] Add all new tokens to `design-tokens.css`
- [ ] Configure dark mode variants
- [ ] Update Tailwind config with token mappings
- [ ] Migrate existing component styles to use new tokens
- [ ] Test across themes (light, dark, system)
- [ ] Verify reduced-motion preferences work
- [ ] Document in Storybook or design system

---

*Last updated: December 26, 2025*
