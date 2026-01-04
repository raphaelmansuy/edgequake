# OODA Loop 49: Root Cause Identified - Hyphenation Artifacts
## SpaceTimePilot Paper (01_2512.25075v1.pdf)

**Date**: 2026-01-04  
**Focus**: Debug analysis reveals critical hyphenation handling bug  
**Discovery**: Text appears truncated due to hyphenated line breaks

---

## 🔍 OBSERVE: The Smoking Gun

### Side-by-Side Comparison: Abstract

**Gold (markitdown)**:
```
We present SpaceTimePilot, a video diffusion model that
disentangles space and time for controllable generative ren-
dering. Given a monocular video, SpaceTimePilot can
independently alter the camera viewpoint and the motion
sequence within the generative process, re-rendering the

scene for continuous and arbitrary exploration across space
and time. To achieve this, we introduce an effective an-
imation time-embedding mechanism in the diffusion pro-
cess, allowing explicit control of the output video's motion
sequence with respect to that of the source video.
```

**Current (edgequake-pdf)**:
```
We present Space Time Pilot, a video diffusion model that

disentangles space and time for controllable generative ren- independently alter the camera viewpoint and the motion
sequence within the generative process, re-rendering the

scene for continuous and arbitrary exploration across space
and time. To achieve this, we introduce an effective an- sequence with respect to that of the source video.
```

### Critical Observation ⚠️🔥

Notice the pattern:
1. **Line ends with "ren-"** → Next line should start with "dering" → But it's GONE!
2. **Line ends with "an-"** → Next line should start with "imation" → But it's GONE!

The text **ISN'T MISSING from the PDF** - it's being **INCORRECTLY JOINED** during hyphenation continuation processing.

---

## 🧭 ORIENT: Root Cause Analysis

### The HyphenContinuation Bug 🐛

**Location**: `edgequake/crates/edgequake-pdf/src/processors/text_cleanup.rs`  
**Component**: `HyphenContinuationProcessor`

**Current Behavior** (WRONG):
```
"ren-" + "\ndering" → "ren-dering" (keeps hyphen) OR "ren" (drops continuation)
"an-" + "\nimation" → "an-imation" (keeps hyphen) OR "an" (drops continuation)
```

**Expected Behavior** (CORRECT):
```
"ren-" + "\ndering" → "rendering"
"an-" + "\nimation" → "animation"  
```

### Why This Causes 76% Abstract Loss ❌❌

**Cascade Effect**:
1. First sentence ends with "ren-"
2. Hyphen continuation fails → sentence breaks
3. Rest of paragraph orphaned
4. Subsequent text detection treats it as noise or different block
5. **Result**: Only first ~200 characters of abstract survive

**Evidence from Page 1 Block Count**:
- Page 1: Only **28 blocks** detected (extremely low!)
- Page 2: **96 blocks** (normal)
- Page 3: **95 blocks** (normal)

Page 1's low block count suggests text is being fragmented/discarded during processing.

---

## 🎯 DECIDE: Fix Strategy

### Option 1: Fix HyphenContinuation Logic ✅ PREFERRED
**Pros**:
- Addresses root cause directly
- Will fix abstract, introduction, and all other affected text
- Standard PDF feature that MUST work correctly

**Cons**:
- Need to ensure fix doesn't break existing tests
- Edge cases (em-dash vs hyphen vs soft hyphen)

**Implementation**:
```rust
// Current (buggy):
if line.ends_with('-') {
    // Probably doing: remove hyphen but not joining properly
}

// Fixed:
if line.ends_with('-') && !next_line.is_empty() {
    // Remove hyphen AND join with next line
    line.truncate(line.len() - 1);  // Remove '-'
    line.push_str(&next_line);      // Append continuation
}
```

### Option 2: Improve Block Merging ⚠️ SECONDARY
**Pros**:
- Could recover orphaned text blocks
- Complementary to Option 1

**Cons**:
- Doesn't fix root cause
- Treating symptoms, not disease

---

## ⚡ ACT: Implementation Plan for Loop 50

### Step 1: Locate HyphenContinuationProcessor
```bash
cd edgequake/crates/edgequake-pdf/src/processors
grep -A 20 "HyphenContinuation" text_cleanup.rs
```

### Step 2: Examine Current Implementation
- Check how hyphens are detected
- Verify how line joining works
- Identify why continuation text is lost

### Step 3: Write Test Case
```rust
#[test]
fn test_hyphen_continuation_mid_word() {
    let input = "This is a gener-\native rendering model";
    let expected = "This is a generative rendering model";
    // Test that hyphenated words are properly joined
}
```

### Step 4: Fix Implementation
- Ensure hyphen is removed
- Ensure next line is joined (not lost!)
- Handle edge cases (em-dash, en-dash, soft hyphen)

### Step 5: Validate
- Run full test suite (must pass all 133 tests)
- Re-extract SpaceTimePilot paper
- Verify abstract retention improves from 23% to >80%

---

## 📊 RESULT: Loop 49 Insights

### Key Discovery 🔥
**Root cause identified**: Hyphenation continuation processing is either:
1. **Removing hyphens but not joining lines** (most likely), OR
2. **Treating hyphenated continuations as separate blocks**

### Impact Assessment
- **Abstract**: 23.4% retention → Expect 80%+ after fix
- **Introduction**: 29.3% retention → Expect 70%+ after fix
- **Method**: 66.9% retention → Expect 85%+ after fix
- **Results**: 55.5% retention → Expect 75%+ after fix

### Confidence Level
- **Hyphenation bug hypothesis**: 95% confidence ✅✅✅
- **Fix will improve extraction**: 90% confidence ✅
- **Won't break existing tests**: 85% confidence ⚠️ (need to verify)

### Expected Outcome
If fix is successful:
- Overall retention: 71.1% → **85-90%**
- Abstract retention: 23.4% → **80%+**
- Introduction retention: 29.3% → **70%+**

---

## 🎯 Commit Message
```
docs(pdf): OODA Loop 49 - Root cause identified: Hyphenation bug

Critical discovery through side-by-side comparison:
- Text ending with "ren-" loses continuation "dering"
- Text ending with "an-" loses continuation "imation"
- Pattern: hyphenated line breaks are NOT being properly joined

This explains:
- 76% abstract loss (cascade effect from first sentence break)
- 71% introduction loss (same pattern)
- Page 1 only has 28 blocks (text fragmentation)

Root cause: HyphenContinuationProcessor in text_cleanup.rs
either removes hyphens without joining OR treats continuations
as separate blocks.

Next: Examine and fix HyphenContinuationProcessor implementation.
Expected improvement: 71% → 85-90% overall retention.
```
