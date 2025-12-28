# Testing & Validation Specification

> Quality assurance guidelines for the chat interface UX upgrade.

## Overview

This document outlines the testing strategy, validation criteria, and acceptance tests for the enhanced query interface.

---

## Testing Scope

### Components Under Test

| Component | File | Tests Required |
|-----------|------|----------------|
| ChatMessage | `chat-message.tsx` | Unit, Visual, A11y |
| CodeBlock | `code-block.tsx` | Unit, Visual, Snapshot |
| ChatInput | `chat-input.tsx` | Unit, Integration, A11y |
| MermaidDiagram | `mermaid-diagram.tsx` | Unit, Visual, Error |
| QueryInterface | `query-interface.tsx` | Integration, E2E |
| MarkdownRenderer | `markdown-renderer.tsx` | Unit, Visual, Snapshot |
| HistoryPanel | `conversation-history-panel.tsx` | Unit, Integration |

---

## Unit Tests

### ChatMessage Component

```typescript
// src/components/query/__tests__/chat-message.test.tsx

describe('ChatMessage', () => {
  describe('User Message', () => {
    it('renders user message with correct styling', () => {
      render(<ChatMessage role="user" content="Hello" />);
      expect(screen.getByRole('article')).toHaveClass('chat-message-user');
    });

    it('displays user avatar with initial', () => {
      render(<ChatMessage role="user" content="Hello" userName="Alice" />);
      expect(screen.getByText('A')).toBeInTheDocument();
    });

    it('shows timestamp on hover', async () => {
      render(<ChatMessage role="user" content="Hello" timestamp="2024-12-26T12:00:00Z" />);
      await userEvent.hover(screen.getByRole('article'));
      expect(screen.getByText('12:00 PM')).toBeVisible();
    });
  });

  describe('Assistant Message', () => {
    it('renders assistant message with markdown', () => {
      render(<ChatMessage role="assistant" content="**Bold** text" />);
      expect(screen.getByRole('article')).toContainHTML('<strong>Bold</strong>');
    });

    it('shows metadata footer when complete', () => {
      render(
        <ChatMessage 
          role="assistant" 
          content="Response" 
          metadata={{ mode: 'hybrid', tokens: 123, duration: 1.5 }}
        />
      );
      expect(screen.getByText('hybrid')).toBeInTheDocument();
      expect(screen.getByText('123 tokens')).toBeInTheDocument();
    });

    it('displays action buttons on hover', async () => {
      render(<ChatMessage role="assistant" content="Response" />);
      await userEvent.hover(screen.getByRole('article'));
      expect(screen.getByRole('button', { name: /copy/i })).toBeVisible();
    });
  });

  describe('Streaming State', () => {
    it('shows typing indicator during streaming', () => {
      render(<ChatMessage role="assistant" content="" isStreaming />);
      expect(screen.getByTestId('typing-indicator')).toBeInTheDocument();
    });

    it('hides metadata during streaming', () => {
      render(<ChatMessage role="assistant" content="Partial" isStreaming />);
      expect(screen.queryByText('tokens')).not.toBeInTheDocument();
    });
  });
});
```

### CodeBlock Component

```typescript
// src/components/query/__tests__/code-block.test.tsx

describe('CodeBlock', () => {
  const sampleCode = `function hello() {\n  console.log("Hello");\n}`;

  it('renders code with syntax highlighting', () => {
    render(<CodeBlock code={sampleCode} language="javascript" />);
    expect(screen.getByRole('code')).toBeInTheDocument();
  });

  it('displays language badge in header', () => {
    render(<CodeBlock code={sampleCode} language="javascript" />);
    expect(screen.getByText('javascript')).toBeInTheDocument();
  });

  it('shows line numbers when enabled', () => {
    render(<CodeBlock code={sampleCode} language="javascript" showLineNumbers />);
    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('copies code to clipboard on button click', async () => {
    const mockClipboard = { writeText: vi.fn() };
    Object.assign(navigator, { clipboard: mockClipboard });
    
    render(<CodeBlock code={sampleCode} language="javascript" />);
    await userEvent.click(screen.getByRole('button', { name: /copy/i }));
    
    expect(mockClipboard.writeText).toHaveBeenCalledWith(sampleCode);
  });

  it('shows success state after copy', async () => {
    render(<CodeBlock code={sampleCode} language="javascript" />);
    await userEvent.click(screen.getByRole('button', { name: /copy/i }));
    
    expect(screen.getByRole('button', { name: /copied/i })).toBeInTheDocument();
  });

  it('collapses long code blocks by default', () => {
    const longCode = Array(50).fill('line').join('\n');
    render(<CodeBlock code={longCode} language="text" maxLines={20} />);
    
    expect(screen.getByRole('button', { name: /show more/i })).toBeInTheDocument();
  });
});
```

### ChatInput Component

```typescript
// src/components/query/__tests__/chat-input.test.tsx

describe('ChatInput', () => {
  const onSubmit = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders textarea with placeholder', () => {
    render(<ChatInput onSubmit={onSubmit} />);
    expect(screen.getByPlaceholderText(/ask a question/i)).toBeInTheDocument();
  });

  it('auto-resizes on input', async () => {
    render(<ChatInput onSubmit={onSubmit} />);
    const textarea = screen.getByRole('textbox');
    
    await userEvent.type(textarea, 'Line 1\nLine 2\nLine 3');
    expect(textarea.scrollHeight).toBeGreaterThan(48);
  });

  it('submits on Enter key', async () => {
    render(<ChatInput onSubmit={onSubmit} />);
    const textarea = screen.getByRole('textbox');
    
    await userEvent.type(textarea, 'Hello{enter}');
    expect(onSubmit).toHaveBeenCalledWith('Hello', expect.any(Object));
  });

  it('does not submit on Shift+Enter', async () => {
    render(<ChatInput onSubmit={onSubmit} />);
    const textarea = screen.getByRole('textbox');
    
    await userEvent.type(textarea, 'Line 1{shift}{enter}Line 2');
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('disables send button when empty', () => {
    render(<ChatInput onSubmit={onSubmit} />);
    expect(screen.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  it('shows character count near limit', async () => {
    render(<ChatInput onSubmit={onSubmit} maxLength={100} />);
    const textarea = screen.getByRole('textbox');
    
    await userEvent.type(textarea, 'a'.repeat(85));
    expect(screen.getByText('85 / 100')).toBeInTheDocument();
  });

  it('prevents submission beyond limit', async () => {
    render(<ChatInput onSubmit={onSubmit} maxLength={100} />);
    const textarea = screen.getByRole('textbox');
    
    await userEvent.type(textarea, 'a'.repeat(105));
    expect(screen.getByRole('button', { name: /send/i })).toBeDisabled();
  });
});
```

---

## Integration Tests

### Query Interface Flow

```typescript
// src/components/query/__tests__/query-interface.integration.test.tsx

describe('QueryInterface Integration', () => {
  it('completes a full query flow', async () => {
    render(<QueryInterface />);
    
    // Start with empty state
    expect(screen.getByText(/ask about your knowledge graph/i)).toBeInTheDocument();
    
    // Type and submit query
    const input = screen.getByRole('textbox');
    await userEvent.type(input, 'What are the main entities?{enter}');
    
    // User message appears
    expect(screen.getByText('What are the main entities?')).toBeInTheDocument();
    
    // Loading state shown
    expect(screen.getByTestId('loading-indicator')).toBeInTheDocument();
    
    // Wait for response
    await waitFor(() => {
      expect(screen.queryByTestId('loading-indicator')).not.toBeInTheDocument();
    }, { timeout: 10000 });
    
    // Response appears
    expect(screen.getByText(/main entities/i)).toBeInTheDocument();
  });

  it('handles streaming responses', async () => {
    // Mock streaming response
    server.use(
      rest.post('/api/query', (req, res, ctx) => {
        return res(ctx.stream(async function* () {
          yield 'The ';
          await delay(50);
          yield 'main ';
          await delay(50);
          yield 'entities are...';
        }));
      })
    );
    
    render(<QueryInterface />);
    
    await userEvent.type(screen.getByRole('textbox'), 'Query{enter}');
    
    // Check streaming text appears progressively
    await waitFor(() => {
      expect(screen.getByText(/The main/)).toBeInTheDocument();
    });
  });

  it('handles errors gracefully', async () => {
    server.use(
      rest.post('/api/query', (req, res, ctx) => {
        return res(ctx.status(500), ctx.json({ error: 'LLM error' }));
      })
    );
    
    render(<QueryInterface />);
    
    await userEvent.type(screen.getByRole('textbox'), 'Query{enter}');
    
    await waitFor(() => {
      expect(screen.getByText(/error/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /retry/i })).toBeInTheDocument();
    });
  });
});
```

---

## End-to-End Tests (Playwright)

### Query Flow E2E

```typescript
// e2e/query-flow.spec.ts

import { test, expect } from '@playwright/test';

test.describe('Query Page', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/query');
  });

  test('displays empty state initially', async ({ page }) => {
    await expect(page.getByText(/ask about your knowledge graph/i)).toBeVisible();
    await expect(page.getByRole('button', { name: /send/i })).toBeDisabled();
  });

  test('submits query and receives response', async ({ page }) => {
    // Type query
    await page.getByPlaceholder(/ask a question/i).fill('What entities are in my graph?');
    
    // Send button becomes enabled
    await expect(page.getByRole('button', { name: /send/i })).toBeEnabled();
    
    // Submit
    await page.getByRole('button', { name: /send/i }).click();
    
    // User message appears
    await expect(page.getByText('What entities are in my graph?')).toBeVisible();
    
    // Wait for response (adjust timeout based on API)
    await expect(page.locator('.assistant-message')).toBeVisible({ timeout: 30000 });
  });

  test('copy button works on code blocks', async ({ page }) => {
    // Submit query that will return code
    await page.getByPlaceholder(/ask a question/i).fill('Show me a code example');
    await page.keyboard.press('Enter');
    
    // Wait for response with code block
    await expect(page.locator('.code-block')).toBeVisible({ timeout: 30000 });
    
    // Click copy button
    await page.locator('.code-block').getByRole('button', { name: /copy/i }).click();
    
    // Verify copied state
    await expect(page.getByRole('button', { name: /copied/i })).toBeVisible();
  });

  test('keyboard navigation works', async ({ page }) => {
    // Tab to input
    await page.keyboard.press('Tab');
    await expect(page.getByPlaceholder(/ask a question/i)).toBeFocused();
    
    // Type and submit with Enter
    await page.keyboard.type('Hello');
    await page.keyboard.press('Enter');
    
    // Shift+Tab should move focus back
    await page.keyboard.press('Shift+Tab');
    await expect(page.getByRole('button', { name: /mode/i })).toBeFocused();
  });
});
```

### Mobile Responsiveness E2E

```typescript
// e2e/query-mobile.spec.ts

import { test, expect, devices } from '@playwright/test';

test.use({ ...devices['iPhone 13'] });

test.describe('Query Page Mobile', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/query');
  });

  test('renders mobile layout correctly', async ({ page }) => {
    // Sidebar should be collapsed
    await expect(page.locator('.sidebar')).not.toBeVisible();
    
    // Input should be at bottom with safe area
    const input = page.getByPlaceholder(/ask a question/i);
    const box = await input.boundingBox();
    expect(box!.y).toBeGreaterThan(page.viewportSize()!.height * 0.7);
  });

  test('hamburger menu opens sidebar', async ({ page }) => {
    await page.getByRole('button', { name: /menu/i }).click();
    await expect(page.locator('.sidebar')).toBeVisible();
  });

  test('history opens as bottom sheet', async ({ page }) => {
    await page.getByRole('button', { name: /history/i }).click();
    
    // Bottom sheet should slide up
    const sheet = page.locator('.bottom-sheet');
    await expect(sheet).toBeVisible();
    
    // Can swipe down to close
    await sheet.swipe('down');
    await expect(sheet).not.toBeVisible();
  });

  test('touch interactions work', async ({ page }) => {
    // Long press to select message
    const message = page.locator('.assistant-message').first();
    await message.tap({ timeout: 500 }); // Long press
    
    // Context menu should appear
    await expect(page.locator('.message-actions')).toBeVisible();
  });
});
```

---

## Accessibility Tests

### Automated A11y Scan

```typescript
// src/components/query/__tests__/accessibility.test.tsx

import { axe, toHaveNoViolations } from 'jest-axe';

expect.extend(toHaveNoViolations);

describe('Accessibility', () => {
  it('ChatMessage has no accessibility violations', async () => {
    const { container } = render(
      <ChatMessage role="assistant" content="Hello world" />
    );
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('CodeBlock has no accessibility violations', async () => {
    const { container } = render(
      <CodeBlock code="const x = 1;" language="javascript" />
    );
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('ChatInput has no accessibility violations', async () => {
    const { container } = render(
      <ChatInput onSubmit={() => {}} />
    );
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });

  it('QueryInterface has no accessibility violations', async () => {
    const { container } = render(<QueryInterface />);
    const results = await axe(container);
    expect(results).toHaveNoViolations();
  });
});
```

### Manual A11y Checklist

- [ ] All interactive elements focusable via keyboard
- [ ] Focus order follows visual layout
- [ ] Focus visible indicator on all elements
- [ ] Skip links to main content
- [ ] Proper heading hierarchy (h1 → h2 → h3)
- [ ] Images/icons have alt text or aria-label
- [ ] Form fields have associated labels
- [ ] Error states announced to screen readers
- [ ] Live regions for dynamic content updates
- [ ] Color contrast meets WCAG AA (4.5:1 text, 3:1 UI)
- [ ] Reduced motion preference respected
- [ ] Text resizable up to 200% without loss
- [ ] Touch targets minimum 44x44px on mobile

---

## Visual Regression Tests

### Storybook + Chromatic

Configure stories for visual testing:

```typescript
// src/components/query/ChatMessage.stories.tsx

import type { Meta, StoryObj } from '@storybook/react';
import { ChatMessage } from './chat-message';

const meta: Meta<typeof ChatMessage> = {
  title: 'Query/ChatMessage',
  component: ChatMessage,
  parameters: {
    chromatic: { viewports: [375, 768, 1440] },
  },
};

export default meta;
type Story = StoryObj<typeof ChatMessage>;

export const UserMessage: Story = {
  args: {
    role: 'user',
    content: 'What are the main entities in my knowledge graph?',
    timestamp: '2024-12-26T12:00:00Z',
  },
};

export const AssistantMessage: Story = {
  args: {
    role: 'assistant',
    content: '## Main Entities\n\nThe knowledge graph contains:\n\n1. **SARAH_CHEN** - A person\n2. **BITS_PILANI** - An organization',
    metadata: { mode: 'hybrid', tokens: 234, duration: 1.5 },
  },
};

export const StreamingMessage: Story = {
  args: {
    role: 'assistant',
    content: 'The main entities are',
    isStreaming: true,
  },
};

export const WithCodeBlock: Story = {
  args: {
    role: 'assistant',
    content: '```python\ndef hello():\n    print("Hello")\n```',
  },
};

export const WithMermaid: Story = {
  args: {
    role: 'assistant',
    content: '```mermaid\ngraph TD\n    A --> B\n    B --> C\n```',
  },
};

export const DarkMode: Story = {
  args: {
    role: 'assistant',
    content: 'Dark mode message',
  },
  parameters: {
    backgrounds: { default: 'dark' },
  },
  decorators: [
    (Story) => (
      <div className="dark">
        <Story />
      </div>
    ),
  ],
};
```

---

## Performance Testing

### Web Vitals Thresholds

| Metric | Target | Acceptable |
|--------|--------|------------|
| LCP (Largest Contentful Paint) | < 1.5s | < 2.5s |
| FID (First Input Delay) | < 50ms | < 100ms |
| CLS (Cumulative Layout Shift) | < 0.05 | < 0.1 |
| TTI (Time to Interactive) | < 2s | < 3.5s |
| Bundle Size (JS) | < 200KB gzipped | < 300KB |

### Lighthouse Targets

- Performance: > 90
- Accessibility: > 95
- Best Practices: > 90
- SEO: > 90

### Bundle Analysis

```bash
# Analyze bundle
bun run build:analyze

# Expected output
# - Main bundle: < 150KB
# - Mermaid (lazy): ~300KB (loaded on demand)
# - KaTeX (lazy): ~200KB (loaded on demand)
# - Syntax highlighting (lazy): ~100KB (loaded on demand)
```

---

## Test Execution Commands

```bash
# Unit tests
bun test

# Unit tests with coverage
bun test --coverage

# Integration tests
bun test:integration

# E2E tests
bunx playwright test

# E2E tests with UI
bunx playwright test --ui

# A11y tests only
bun test --grep accessibility

# Visual regression (Chromatic)
bunx chromatic --project-token=$CHROMATIC_TOKEN

# Performance audit
bunx lighthouse http://localhost:3000/query --output html
```

---

## CI Pipeline Integration

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: bun install
      - run: bun test --coverage
      - uses: codecov/codecov-action@v3

  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: bun install
      - run: bunx playwright install --with-deps
      - run: bun run build
      - run: bunx playwright test
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: playwright-report
          path: playwright-report

  visual-regression:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v1
      - run: bun install
      - run: bunx chromatic --project-token=${{ secrets.CHROMATIC_TOKEN }}
```

---

## Acceptance Criteria

### Phase 1 Complete When:
- [ ] All unit tests pass (>90% coverage)
- [ ] All components render without errors
- [ ] Basic keyboard navigation works
- [ ] Messages display correctly

### Phase 2 Complete When:
- [ ] All integration tests pass
- [ ] Streaming works without flicker
- [ ] Code blocks copy correctly
- [ ] Mermaid diagrams render

### Phase 3 Complete When:
- [ ] All E2E tests pass
- [ ] Lighthouse scores meet targets
- [ ] A11y audit passes
- [ ] Visual regression approved

---

*Last updated: December 26, 2025*
