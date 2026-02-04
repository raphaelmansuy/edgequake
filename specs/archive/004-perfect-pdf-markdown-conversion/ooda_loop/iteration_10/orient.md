# OODA-10: Orient

## First Principles Analysis

### What is the fundamental problem?

**Block merging treats word boundaries incorrectly.**

When two text blocks are merged in PDF extraction, we need to decide:

1. Should we add a space between them?
2. Should we remove a hyphen?
3. Should we join them directly?

The current heuristics fail because they rely on surface-level character patterns rather than understanding the semantic structure of text.

### Why do the current heuristics fail?

#### Problem 1: Word Fragment Detection

Current logic:

```rust
let is_likely_word_fragment = matches!(
    (last_char, first_char),
    (Some(c1), Some(c2)) if c1.is_alphabetic() && c2.is_lowercase()
) && !self.text.trim_end().ends_with(' ');
```

**Flaw**: This says "alphabetic + lowercase = word continuation"

Counter-examples from English:

- "for whiteboard" → last='r', first='w' → both alphabetic → WRONG
- "the robot" → last='e', first='r' → both alphabetic → WRONG
- "human centric" → last='n', first='c' → both alphabetic → WRONG

**First Principle**: A word fragment must be an INCOMPLETE word. English words typically don't end with:

- Articles: "the", "a", "an"
- Prepositions: "for", "to", "in", "on", "at"
- Common short words: "is", "as", "or"

A proper fragment would be something like: "decom-" + "position" where "decom" is not a valid English word.

#### Problem 2: Hyphen Classification

Current logic:

```rust
if ends_with_hyphen && starts_with_lowercase {
    // Remove hyphen and join
}
```

**Flaw**: This removes ALL hyphens, including compound-word hyphens.

**First Principle**: There are TWO types of hyphens:

1. **Continuation hyphen**: Added by typesetter to break long words at line end
   - "modifi-" + "cation" → "modification"
   - "observa-" + "tion" → "observation"
   - Key: The prefix is NOT a valid standalone word

2. **Compound hyphen**: Intentional hyphen in compound words
   - "long-" + "horizon" → "long-horizon"
   - "self-" + "supervised" → "self-supervised"
   - "hand-" + "eye" → "hand-eye"
   - Key: The prefix IS a valid standalone word

**Detection heuristic**:

- If the text before hyphen is a complete English word (exists in dictionary or > 3 letters and pronounceable), keep the hyphen
- If it's a partial word (like "modifi", "techni", "observa"), remove the hyphen

### Typesetting Principles (Donald Knuth)

From TeX and typesetting theory:

1. **Word Breaking**: TeX breaks words at syllable boundaries with a hyphen
   - "ob-ser-va-tion" → any of these breaks is valid
   - The hyphen is INSERTED by the typesetter, not part of the word

2. **Compound Words**: Hyphens in compound words are part of the word
   - "self-aware" → the hyphen is semantically meaningful
   - "state-of-the-art" → hyphens connect independent words

3. **Detection**: Look at the PREFIX before hyphen:
   - "ob-", "ser-", "va-" → partial syllables, not words → continuation
   - "self-", "long-", "high-" → complete words → compound

## Strategic Decision

### Option A: Dictionary-based detection (Complex)

- Load English word list
- Check if prefix is a valid word
- Pros: Accurate
- Cons: Requires dictionary, slow, language-dependent

### Option B: Common pattern matching (Simple)

- Maintain list of common compound prefixes: "self-", "long-", "short-", "hand-", "eye-", etc.
- Keep hyphen for known patterns
- Pros: Fast, no external dependency
- Cons: Won't catch all cases

### Option C: Morphological heuristic (Balanced)

- If prefix length >= 4 AND ends with complete syllable → likely compound word
- If prefix length < 4 → likely continuation
- Common prefixes like "pre-", "re-", "co-" → special handling
- Pros: Good balance of accuracy and simplicity
- Cons: Imperfect but reasonable

**Decision**: Implement **Option C** with enhancements:

1. **For word fragments**:
   - Only treat as fragment if last word is < 3 characters AND looks like partial word
   - Always add space otherwise

2. **For hyphens**:
   - Check if prefix before hyphen is a "complete" morpheme:
     - Known compound prefixes (self-, long-, hand-, high-, low-, etc.)
     - Word length >= 4 with vowel (pronounceable)
   - If complete morpheme → keep hyphen
   - If partial → remove hyphen

## Risk Assessment

| Risk                                      | Impact                  | Mitigation                            |
| ----------------------------------------- | ----------------------- | ------------------------------------- |
| Over-correction: add too many spaces      | Words split incorrectly | Keep tight horizontal proximity check |
| Under-correction: still join words        | Same bugs persist       | Test with known failure cases         |
| Regression: break hyphenation that worked | Quality drops           | Test Qwen.pdf "Pushing" word          |
| Performance impact                        | Slower extraction       | Keep heuristics simple, no dictionary |

## Implementation Plan

1. **Fix word fragment detection** in `Block::merge()`:
   - Extract last word from `self.text`
   - If last word is in common-short-word list → add space
   - If last word length < 3 → check if valid word fragment

2. **Fix hyphen handling** in `Block::merge()`:
   - Extract prefix before hyphen
   - If prefix matches compound pattern → keep hyphen + space
   - Otherwise → remove hyphen and join

3. **Test thoroughly**:
   - v2 PDF: "for whiteboard" should have space
   - v2 PDF: "long-horizon" should keep hyphen
   - Qwen PDF: "Pushing" should remain intact
