# OODA-25 Orient: Root Cause Analysis

## Key Finding: Caption Processing Pipeline Gap

### Current Processing Flow
```
┌─────────────────────────┐
│ CaptionDetectionProcessor │ → Only marks blocks starting with "Figure N:"
└───────────┬─────────────┘
            │ Block 39: "Figure 1.Illustration...reposi-" → BlockType::Caption
            │ Block 40: "tory. The LLM..." → BlockType::Text (NOT marked!)
            ▼
┌─────────────────────────┐
│ HyphenContinuationProcessor │ → Handles hyphenation, but...
└───────────┬─────────────┘
            │ Different block types prevent proper merge
            ▼
┌─────────────────────────┐
│ BlockMergeProcessor     │ → Only merges Text+Text, Header+Header, List+List
└───────────┬─────────────┘
            │ Caption+Text rejected because types don't match
            │ Caption blocks stay fragmented
            ▼
┌─────────────────────────┐
│ render_caption()        │ → Outputs *text* (italics)
└─────────────────────────┘
```

## Root Causes

### Root Cause 1: Caption Continuation Not Detected
- CaptionDetectionProcessor regex: `^(Figure|Fig\.|Table|Tab\.)\s*\d+[:.]]`
- Only matches the FIRST block of a caption
- Second block "tory. The LLM..." doesn't match → stays Text

### Root Cause 2: BlockMerge Type Restriction
- Line 211-219 in layout_processing.rs:
  ```rust
  if !matches!(a.block_type, BlockType::Text | BlockType::SectionHeader | BlockType::ListItem)
  ```
- Caption type not in list → never merged

### Root Cause 3: Italics vs Blockquote Format
- render_caption() outputs: `*{}*` (italics)
- Gold file uses: `> **Figure 1:** description` (blockquote with bold label)

## First Principles Solution

### Option A: Extend CaptionDetectionProcessor ✅ PREFERRED
- After marking a Caption, check if the NEXT block:
  - Has similar X position (same column)
  - Is immediately adjacent vertically
  - Contains continuation of sentence (lowercase start, hyphenation)
- Mark it as Caption too

**WHY preferred:** 
- Keeps caption detection logic cohesive
- No need to modify BlockMergeProcessor
- Follows Single Responsibility Principle

### Option B: Allow Caption+Text in BlockMerge
- Add Caption to mergeable types
- Let existing merge logic handle it

**WHY not preferred:**
- Caption+Text merge could be too aggressive
- May merge unrelated text into captions
- Violates SRP (merge processor shouldn't know about caption semantics)

### Option C: Dedicated CaptionContinuationProcessor
- New processor specifically for caption continuation
- Runs after CaptionDetection, before BlockMerge

**WHY not preferred:**
- Adds another processor
- More complexity than needed

## Selected Approach: Option A + Format Fix

1. Extend `CaptionDetectionProcessor` to detect continuation blocks
2. Update `render_caption()` to use blockquote format with bold label
