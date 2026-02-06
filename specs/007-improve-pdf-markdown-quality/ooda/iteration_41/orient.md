# IT41 — Orient: Distinguish URL Punctuation from General Punctuation

## Root Cause

IT40's 22% threshold for proportional fonts is correct for word boundaries but too aggressive for URL/path punctuation which has wider kerning.

## Key Insight: Punctuation Has Different Roles

**URL/Path Punctuation** (bind tightly to adjacent chars):
- `:` — protocol separator (https:)
- `/` — path separator (/path/to)
- `.` — domain/file extension (github.com, file.txt)
- `@` — email separator (user@example.com)
- `-` `_` — word connectors in identifiers (my-file, my_var)

**General Punctuation** (typically word boundaries):
- `&` — "A & B" should have spaces
- `,` — "one, two" has space after
- `;` — semicolon ends clauses
- `!` `?` — sentence endings
- `(` `)` — often have spaces around

## Solution Strategy

Apply different thresholds based on punctuation type:

```rust
fn is_url_punctuation(c: char) -> bool {
    matches!(c, ':' | '/' | '.' | '@' | '-' | '_')
}

// URL punctuation: 33% threshold (avoid splitting URLs)
// General punctuation: 22% threshold (allow word boundaries)
```

## Why This Works

1. **URLs remain intact**: `https://github.com` — all chars use 33% threshold
2. **Normal text splits correctly**: "A & B" — ampersand uses 22%, allows spacing
3. **Filenames work**: `file.txt` — period uses 33% threshold
4. **Sentences work**: "Hello. World" — period uses 33%, but capital 'W' has large gap anyway

## Alternative Considered: Post-Processing URL Cleanup

**Rejected** because:
- Would require URL regex matching in markdown output
- Treating symptoms rather than root cause
- Would miss edge cases like custom protocols
