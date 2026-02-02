# OODA-10: Decide

## Decision Summary

Fix the `Block::merge()` function to correctly handle:
1. Word boundaries (add spaces between separate words)
2. Compound-word hyphens (preserve hyphens in "long-horizon", "self-supervised")
3. Continuation hyphens (remove hyphens in "modifi-" + "cation")

## Specific Changes

### Change 1: Fix Word Fragment Detection

**File**: `src/schema/block.rs`  
**Location**: Lines 339-349

**Current code** (BUGGY):
```rust
let is_likely_word_fragment = matches!(
    (last_char, first_char),
    (Some(c1), Some(c2)) if c1.is_alphabetic() && c2.is_lowercase()
) && !self.text.trim_end().ends_with(' ');

if is_likely_word_fragment {
    self.text = self.text.trim_end().to_string();
    self.text.push_str(other.text.trim_start());
}
```

**New code**:
```rust
// OODA-10: More conservative word fragment detection
// WHY: Only join without space if the last "word" is clearly a partial fragment
// Common words like "for", "the", "is" should NOT be treated as fragments
let last_word = self.text.split_whitespace().last().unwrap_or("");
let is_complete_common_word = matches!(
    last_word.to_lowercase().as_str(),
    "the" | "a" | "an" | "for" | "to" | "in" | "on" | "at" | "of" | "by" |
    "is" | "as" | "or" | "and" | "but" | "so" | "if" | "it" | "we" | "be" |
    "this" | "that" | "with" | "from" | "are" | "was" | "has" | "had" | "not"
);

let is_likely_word_fragment = if is_complete_common_word {
    false  // Never treat common words as fragments
} else {
    // Only fragments if very short partial word AND looks incomplete
    let is_very_short = last_word.len() <= 2;
    let ends_alpha_lowercase = matches!(last_char, Some(c) if c.is_lowercase());
    is_very_short && ends_alpha_lowercase && !self.text.trim_end().ends_with(' ')
};
```

### Change 2: Fix Compound Hyphen Handling

**File**: `src/schema/block.rs`  
**Location**: Lines 330-335

**Current code** (BUGGY):
```rust
if ends_with_hyphen && starts_with_lowercase {
    // Explicit hyphenation: remove hyphen and join
    self.text = self.text.trim_end_matches('-').trim_end().to_string();
    self.text.push_str(other.text.trim_start());
}
```

**New code**:
```rust
if ends_with_hyphen && starts_with_lowercase {
    // OODA-10: Distinguish continuation hyphen vs compound hyphen
    // WHY: "modifi-" + "cation" → "modification" (continuation)
    //      "long-" + "horizon" → "long-horizon" (compound)
    let prefix = self.text.trim_end().trim_end_matches('-');
    let last_word = prefix.split_whitespace().last().unwrap_or("");
    
    // Check if prefix is a known compound word prefix (keep hyphen)
    let is_compound_prefix = matches!(
        last_word.to_lowercase().as_str(),
        "long" | "short" | "self" | "hand" | "eye" | "high" | "low" | "well" |
        "full" | "half" | "co" | "pre" | "re" | "anti" | "non" | "multi" |
        "cross" | "whole" | "end" | "real" | "time" | "data" | "user" |
        "loco" | "semi" | "all" | "one" | "two" | "three" | "first" | "second"
    );
    
    // Also treat as compound if prefix has >= 4 chars AND contains vowel
    // (i.e., is likely a complete word, not "modifi" or "techni")
    let has_vowel = last_word.chars().any(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'));
    let is_likely_complete_word = last_word.len() >= 4 && has_vowel && 
        !last_word.to_lowercase().ends_with("ti") &&  // "observa-ti-on" fragments
        !last_word.to_lowercase().ends_with("ni") &&  // "tech-ni-cal" fragments
        !last_word.to_lowercase().ends_with("fi");    // "modi-fi-ed" fragments
    
    if is_compound_prefix || is_likely_complete_word {
        // Keep hyphen, add space (compound word)
        self.text.push(' ');
        if !self.spans.is_empty() || !other.spans.is_empty() {
            self.spans.push(TextSpan::plain(" "));
        }
        self.text.push_str(&other.text);
    } else {
        // Remove hyphen, join directly (continuation)
        self.text = self.text.trim_end_matches('-').trim_end().to_string();
        self.text.push_str(other.text.trim_start());
    }
}
```

## Test Cases

| Input | Expected | Current | After Fix |
|-------|----------|---------|-----------|
| "for" + "whiteboard" | "for whiteboard" | "forwhiteboard" | "for whiteboard" |
| "long-" + "horizon" | "long-horizon" | "longhorizon" | "long-horizon" |
| "self-" + "supervised" | "self-supervised" | "selfsupervised" | "self-supervised" |
| "modifi-" + "cation" | "modification" | "modification" | "modification" |
| "observa-" + "tion" | "observation" | "observation" | "observation" |
| "Pushing" | "Pushing" | "Pushing" | "Pushing" |

## Risks and Mitigations

1. **Risk**: May add unwanted spaces in some edge cases
   - **Mitigation**: Test with full quality suite

2. **Risk**: May keep hyphens that should be removed
   - **Mitigation**: Endings "-ti", "-ni", "-fi" detected as fragments

3. **Risk**: Regression on Qwen.pdf
   - **Mitigation**: Explicit test for "Pushing" word preservation

## Implementation Order

1. Apply changes to `src/schema/block.rs`
2. Run quick smoke test
3. Test v2 PDF specifically for "for whiteboard" and "long-horizon"
4. Test Qwen PDF for "Pushing"
5. Run comprehensive quality tests
6. Document results in act.md
