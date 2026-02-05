# IT13 Orient: Code Block False Positive Analysis

## Root Cause Analysis

### Problem Statement
Email addresses and URLs are being incorrectly detected as code blocks:

**Observed Output:**
```markdown
```
zrguo101@hku.hk aka_xia@foxmail.com chaohuang75@gmail.com
```

```
https://arxiv.
```
```

**Expected Output:**
These should be plain text, not code blocks.

### Why This Happens

```
┌─────────────────────────────────────────────────────────────┐
│           CODE BLOCK DETECTION FLOW                         │
├─────────────────────────────────────────────────────────────┤
│  1. PDF extracts text with font metadata                    │
│  2. looks_like_code() checks font family for monospace      │
│  3. CodeBlockDetectionProcessor marks as BlockType::Code    │
│  4. Renderer outputs ``` fenced block                       │
└─────────────────────────────────────────────────────────────┘

         CURRENT LOGIC (Font-Only)
         ━━━━━━━━━━━━━━━━━━━━━━━━━
         
         Is font monospace?  ───────► Mark as Code
              │
              └─── PROBLEM: No content validation!
```

### Font Detection in LightRAG PDF

The email addresses in academic papers are often rendered in a monospace or
typewriter font (common for author affiliations), which triggers false positives.

### Code vs Non-Code Content Patterns

| Pattern | Is Code? | Why |
|---------|----------|-----|
| `email@domain.com` | NO | Simple email address |
| `https://url.com` | NO | Simple URL |
| `function foo()` | YES | Contains programming syntax |
| `import os` | YES | Programming statement |
| `x = 5` | YES | Variable assignment |
| `{json: "data"}` | YES | Data structure syntax |

### Solution Strategy

**Add content-based filtering** to exclude non-code patterns:

1. **Email Pattern**: `\S+@\S+\.\S+`
2. **URL Pattern**: `https?://` or `www.`
3. **Require Code Indicators**: 
   - Programming keywords (def, function, import, class, etc.)
   - Syntax characters ({, }, [, ], =>, etc.)
   - Assignment operators (=, :=)
   - Line length > threshold for single-line blocks

### First Principles

**What is code?**
- Programming language syntax
- Configuration files with structured syntax
- Command-line instructions
- Data structures (JSON, YAML, etc.)

**What is NOT code?**
- Email addresses (even in monospace font)
- URLs (standalone, not in script context)
- Plain text that happens to use monospace for styling

### Quality Impact

- **Current**: False positive code blocks → Confusing LLM context
- **After Fix**: Clean text extraction → Better RAG quality

## Decision Framework

```
┌─────────────────────────────────────────────────────────────┐
│        PROPOSED CODE DETECTION ALGORITHM                    │
├─────────────────────────────────────────────────────────────┤
│  IF font is monospace:                                      │
│    1. Check content against exclusion patterns              │
│       - Is it ONLY email addresses? → NOT CODE              │
│       - Is it ONLY URLs? → NOT CODE                         │
│    2. Check for positive code indicators                    │
│       - Contains programming keywords? → CODE               │
│       - Contains syntax markers? → CODE                     │
│       - Multi-line with consistent indentation? → CODE      │
│    3. Default: Trust font (monospace = code)                │
└─────────────────────────────────────────────────────────────┘
```

## Files to Modify

1. `src/processors/structure_detection.rs`: Add content filtering to CodeBlockDetectionProcessor
2. `src/schema/block.rs`: May add helper functions for content pattern detection
