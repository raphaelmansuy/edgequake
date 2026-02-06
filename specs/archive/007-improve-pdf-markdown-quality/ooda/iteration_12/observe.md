# IT12 Observe: List Formatting and Nested Lists

## Mission Reminder

- Quality Targets: Multi-column (60→85), Tables (50→80), Lists (55→85), Code (70→90)
- Focus: Lists (nested) currently at 55/100, target 85/100

## Current State Check

### Table 4 - Verified Working ✅

After IT10 fix, Table 4 (simple 5-column table) properly renders:

```markdown
| Statistics   | Agriculture | CS        | Legal     | Mix     |
| ------------ | ----------- | --------- | --------- | ------- |
| Total Tokens | 2,017,886   | 2,306,535 | 5,081,069 | 619,009 |
```

### Tables 1, 2, 3, 5 - Complex Comparison Tables ❌

These are 8-column tables with nested comparison baselines (NaiveRAG vs LightRAG, etc.)
Data is extracted but scattered as prose, not formatted.
**Status**: DEFERRED - Requires spatial analysis overhaul

### List Formatting Issues

Let me check how lists are currently formatted in the output:

```bash
grep -E "^-|^\*|^[0-9]+\." lighrag_2410.05779v3.md | head -30
```

Results:

- Basic bullet lists: Generally work
- Nested lists: Indentation may be flattened
- Numbered lists: May lose numbering context

## Key Files for List Processing

1. **processors/list_processor.rs** - If it exists
2. **renderers/markdown.rs** - Rendering logic
3. **schema/block.rs** - BlockType::List, BulletItem

## Areas to Investigate

1. How are lists detected?
2. How is nesting preserved?
3. What happens to numbered vs bullet lists?
4. Edge cases: mixed lists, continuation paragraphs
