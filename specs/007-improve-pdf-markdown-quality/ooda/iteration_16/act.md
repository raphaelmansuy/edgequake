```markdown
# OODA Iteration 16 - Act

## Actions Taken

### 1. Implemented `join_broken_lines()` Function

**File:** `src/renderers/markdown.rs` (lines 1061-1074)

Fixed-point loop that calls `join_broken_lines_single_pass()` until no more joins are possible.

### 2. Implemented `join_broken_lines_single_pass()` 

**File:** `src/renderers/markdown.rs` (lines 1076-1165)

Single-pass line joiner that:
- Detects word breaks across adjacent lines
- Handles breaks across empty lines (from render_text \n\n suffix)
- Skips code fences, empty lines, markdown structural elements
- Checks for broken words BEFORE checking if next line is structural
  (handles "- based" that looks like list item but is "sockets-based")

### 3. Implemented `should_join_lines()` Detection

**File:** `src/renderers/markdown.rs` (lines 1203-1276)

Three rules:
1. **Lowercase continuation**: prev ends with lowercase, next starts with lowercase (no sentence punctuation)
2. **Trailing hyphen**: prev ends with `word-`, next starts with lowercase
3. **Leading hyphen**: next starts with `- word` or `-word` when prev ends with lowercase

### 4. Implemented `join_two_lines()` Merger

**File:** `src/renderers/markdown.rs` (lines 1278-1329)

Three cases:
1. Next starts with hyphen: merge as "prev-word" 
2. Prev ends with hyphen: check compound prefix list, keep or remove hyphen
3. No hyphen: direct concatenation (word split without hyphen)

### 5. Helper Functions

- `is_markdown_structural_line()` (line 1167): Headers, lists, blockquotes, tables, horizontal rules
- `is_code_fence()` (line 1198): Detects ``` and ~~~ markers

### 6. Integrated into Cleanup Pipeline

Called from `cleanup_markdown_artifacts()` BEFORE TOC cleanup and bold-to-headers, since it affects raw text flow.

## Results

### Before (Apple-Sandbox-Guide-v1.0.pdf)

```
- kSBXProfileNoInternet : TCP/IP netw
orking is prohibited.
- kSBXProfileNoNetwork : All sockets
              - based networking is prohibited.
```

### After

```
- kSBXProfileNoInternet : TCP/IP networking is prohibited.
- kSBXProfileNoNetwork : All sockets-based networking is prohibited.
```

### Test Results

```
test result: ok. 549 passed; 0 failed; 0 ignored
```

## Verification

1. ✅ Mid-word breaks joined ("netw" + "orking" → "networking")
2. ✅ Hyphenated words reconstructed ("sockets" + "- based" → "sockets-based")
3. ✅ Sentence boundaries preserved
4. ✅ Markdown structure preserved (lists, headers, code blocks)
5. ✅ Compound word hyphens preserved ("well-known")
6. ✅ All 549 tests passing
```
