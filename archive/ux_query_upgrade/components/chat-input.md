# ChatInput Component Specification

> Polished input area with auto-resize, character count, keyboard shortcuts, and attachment support.

## Overview

The ChatInput component provides a refined input experience for composing queries with features like auto-resizing textarea, character count, mode selection, and clear affordances.

---

## Visual Design

### Default State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Ask a question...                                               [→]  │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  [🔍 Local] [🌐 Global] [⚡ Hybrid] [📝 Simple]    Enter ↵ · Shift+Enter ⏎  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Focus State

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ What entities are connected to BITS Pilani?             45 chars [→] │  │
│  │                                                                       │  │
│  │ ▋                                                                     │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│       ↑ Focus ring: 3px primary glow                                        │
│  [🔍 Local] [🌐 Global] [⚡ Hybrid] [📝 Simple]    Enter to send            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### With Content (Multi-line)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │ Can you show me:                                                      │  │
│  │                                                                       │  │
│  │ 1. All entities related to RAG concepts                               │  │
│  │ 2. How they connect to organizations                                  │  │
│  │ 3. A code example for querying                                        │  │
│  │                                                                128 [→] │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│  [🔍 Local] [🌐 Global] [⚡ Hybrid] [📝 Simple]    Enter to send            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Component Structure

```tsx
interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  disabled?: boolean;
  loading?: boolean;
  maxLength?: number;
  showCharCount?: boolean;
  showModeSelector?: boolean;
  mode?: QueryMode;
  onModeChange?: (mode: QueryMode) => void;
  placeholder?: string;
  onStop?: () => void;
}

type QueryMode = 'local' | 'global' | 'hybrid' | 'simple';

function ChatInput({
  value,
  onChange,
  onSubmit,
  disabled = false,
  loading = false,
  maxLength = 4000,
  showCharCount = true,
  showModeSelector = true,
  mode = 'hybrid',
  onModeChange,
  placeholder = 'Ask a question...',
  onStop,
}: ChatInputProps) {
  // Implementation
}
```

---

## Sections

### 1. Input Container
- Rounded corners: `rounded-2xl`
- Border: `border border-border`
- Shadow: `shadow-sm`
- Focus ring: `focus-within:border-primary focus-within:shadow-[0_0_0_3px_rgba(var(--primary),0.1)]`

### 2. Textarea
- Auto-resize based on content
- Min height: `48px` (single line)
- Max height: `200px` (then scroll)
- Padding: `px-4 py-3`
- No visible resize handle

### 3. Send Button
- Position: Absolute, bottom-right
- Size: `h-8 w-8`
- Icon: Send (or Stop if loading)
- Disabled state: Reduced opacity

### 4. Character Count
- Position: Before send button
- Shows only when typing
- Format: Just the number
- Color: Muted, warning at 90%

### 5. Mode Selector (Below Input)
- Horizontal toggle group
- Active state: Filled background
- Icons with labels

### 6. Hint Text
- Position: Right side below input
- Text: "Enter to send · Shift+Enter for new line"

---

## States

### Empty
- Placeholder visible
- Send button disabled
- Character count hidden

### Has Content
- Send button enabled
- Character count visible (after threshold)
- Submit on Enter

### Loading (Streaming)
- Stop button instead of Send
- Input may be disabled
- Pulse animation on button

### Disabled
- Reduced opacity
- No interactions
- Cursor: not-allowed

### Error
- Red border
- Error message below
- Shake animation

### Near Limit
- Character count turns warning color (>90%)
- Subtle pulsing

---

## Animations

### Focus Ring
```css
.chat-input-wrapper {
  transition: border-color 0.2s, box-shadow 0.2s;
}

.chat-input-wrapper:focus-within {
  border-color: var(--primary);
  box-shadow: 0 0 0 3px rgb(var(--primary) / 0.1);
}
```

### Auto-Resize
```tsx
const textareaRef = useRef<HTMLTextAreaElement>(null);

useEffect(() => {
  const textarea = textareaRef.current;
  if (textarea) {
    textarea.style.height = 'auto';
    textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
  }
}, [value]);
```

### Button Hover
```css
.send-button {
  transition: transform 0.15s, background-color 0.15s;
}

.send-button:hover:not(:disabled) {
  transform: scale(1.05);
}

.send-button:active:not(:disabled) {
  transform: scale(0.95);
}
```

### Error Shake
```css
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-4px); }
  75% { transform: translateX(4px); }
}

.chat-input-error {
  animation: shake 0.3s ease-in-out;
}
```

---

## Keyboard Handling

| Key | Action |
|-----|--------|
| `Enter` | Submit (if content) |
| `Shift+Enter` | New line |
| `Escape` | Clear input / Blur |
| `Cmd/Ctrl+Enter` | Force submit |

```tsx
const handleKeyDown = (e: React.KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    if (value.trim() && !disabled && !loading) {
      onSubmit();
    }
  }
  
  if (e.key === 'Escape') {
    if (value) {
      onChange('');
    } else {
      (e.target as HTMLTextAreaElement).blur();
    }
  }
};
```

---

## Accessibility

- Role: `form` for the container
- Textarea: aria-label="Query input"
- Send button: aria-label="Send message" or "Stop generating"
- Mode selector: `role="radiogroup"` with `aria-label`
- Character count: aria-live="polite"
- Focus management: Return focus after submit

---

## Responsive Behavior

### Desktop (≥1024px)
- Max width: `max-w-3xl` centered
- Full mode selector visible
- Hint text visible

### Tablet (768px - 1023px)
- Max width: `max-w-2xl`
- Mode selector with icons only
- Hint text hidden

### Mobile (<768px)
- Full width with padding
- Mode selector: Horizontal scroll
- Larger touch targets (48px)
- Hint text hidden
- Safe area inset for bottom

```css
@media (max-width: 767px) {
  .chat-input-container {
    padding-bottom: max(var(--space-4), env(safe-area-inset-bottom));
  }
  
  .send-button {
    min-width: 48px;
    min-height: 48px;
  }
}
```

---

## Implementation

```tsx
import { memo, useCallback, useEffect, useRef } from 'react';
import { Send, StopCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { QueryModeSelector } from './query-mode-selector';

const ChatInput = memo(function ChatInput({
  value,
  onChange,
  onSubmit,
  disabled = false,
  loading = false,
  maxLength = 4000,
  showCharCount = true,
  showModeSelector = true,
  mode = 'hybrid',
  onModeChange,
  placeholder = 'Ask a question...',
  onStop,
}: ChatInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const hasContent = value.trim().length > 0;
  const charCount = value.length;
  const isNearLimit = charCount > maxLength * 0.9;
  const isOverLimit = charCount > maxLength;

  // Auto-resize textarea
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, [value]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (hasContent && !disabled && !loading && !isOverLimit) {
        onSubmit();
      }
    }
    if (e.key === 'Escape') {
      if (value) {
        onChange('');
      } else {
        (e.target as HTMLTextAreaElement).blur();
      }
    }
  }, [hasContent, disabled, loading, isOverLimit, value, onChange, onSubmit]);

  const handleSubmit = useCallback(() => {
    if (loading && onStop) {
      onStop();
    } else if (hasContent && !disabled && !isOverLimit) {
      onSubmit();
    }
  }, [loading, onStop, hasContent, disabled, isOverLimit, onSubmit]);

  return (
    <div className="chat-input-container sticky bottom-0 
                   bg-background/95 backdrop-blur-sm 
                   border-t border-border/40 p-4">
      <form 
        className="chat-input-form max-w-3xl mx-auto"
        onSubmit={(e) => { e.preventDefault(); handleSubmit(); }}
        aria-label="Query form"
      >
        {/* Input Wrapper */}
        <div className={cn(
          "chat-input-wrapper relative rounded-2xl",
          "border border-border bg-card shadow-sm",
          "transition-all duration-200",
          "focus-within:border-primary",
          "focus-within:shadow-[0_0_0_3px_rgba(var(--primary),0.1)]",
          disabled && "opacity-50 cursor-not-allowed",
          isOverLimit && "border-destructive"
        )}>
          {/* Textarea */}
          <textarea
            ref={textareaRef}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={disabled}
            placeholder={placeholder}
            rows={1}
            className={cn(
              "w-full resize-none bg-transparent",
              "px-4 py-3 pr-20",
              "text-foreground placeholder:text-muted-foreground",
              "focus:outline-none",
              "min-h-[48px] max-h-[200px]",
              "disabled:cursor-not-allowed"
            )}
            aria-label="Query input"
            aria-describedby="input-hint"
          />

          {/* Character Count */}
          {showCharCount && hasContent && (
            <span className={cn(
              "absolute right-14 bottom-3",
              "text-xs transition-opacity",
              isNearLimit ? "text-warning" : "text-muted-foreground",
              isOverLimit && "text-destructive"
            )}>
              {charCount}
            </span>
          )}

          {/* Send/Stop Button */}
          <Button
            type="submit"
            size="icon"
            className={cn(
              "absolute right-2 bottom-2 h-8 w-8 rounded-xl",
              "transition-all duration-200",
              loading 
                ? "bg-destructive text-destructive-foreground" 
                : "bg-primary text-primary-foreground",
              !hasContent && !loading && "opacity-40 cursor-not-allowed",
              "hover:scale-105 active:scale-95"
            )}
            disabled={!hasContent && !loading}
            aria-label={loading ? "Stop generating" : "Send message"}
          >
            {loading ? (
              <StopCircle className="h-4 w-4" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>

        {/* Footer: Mode Selector + Hint */}
        <div className="flex items-center justify-between mt-2 px-1">
          {showModeSelector && (
            <QueryModeSelector 
              mode={mode} 
              onModeChange={onModeChange} 
            />
          )}
          <span 
            id="input-hint"
            className="text-xs text-muted-foreground hidden sm:block"
          >
            Enter to send · Shift+Enter for new line
          </span>
        </div>
      </form>
    </div>
  );
});

export default ChatInput;
```

---

## CSS Classes Summary

```css
/* Container */
.chat-input-container {
  @apply sticky bottom-0 
         bg-background/95 backdrop-blur-sm 
         border-t border-border/40 p-4;
}

/* Form wrapper */
.chat-input-form {
  @apply max-w-3xl mx-auto;
}

/* Input wrapper */
.chat-input-wrapper {
  @apply relative rounded-2xl 
         border border-border bg-card shadow-sm
         transition-all duration-200
         focus-within:border-primary
         focus-within:shadow-[0_0_0_3px_rgba(var(--primary),0.1)];
}

/* Textarea */
.chat-input-textarea {
  @apply w-full resize-none bg-transparent
         px-4 py-3 pr-20
         text-foreground placeholder:text-muted-foreground
         focus:outline-none
         min-h-[48px] max-h-[200px];
}

/* Send button */
.chat-send-button {
  @apply absolute right-2 bottom-2 h-8 w-8 rounded-xl
         bg-primary text-primary-foreground
         transition-all duration-200
         hover:scale-105 active:scale-95;
}

/* Character count */
.chat-char-count {
  @apply absolute right-14 bottom-3
         text-xs text-muted-foreground;
}
```

---

*Last updated: December 26, 2025*
