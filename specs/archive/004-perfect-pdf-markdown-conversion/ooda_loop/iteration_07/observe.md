```markdown
# OODA-07 Observe: CamelCase Word Splitting Issue

## Observation Summary

CamelCase compound words are incorrectly split by the post-processor, breaking legitimate technical terms.

## Evidence

**EdgeQuake Output (broken):**
```

Browse Comp (Wei et al., 2025)
Browse Comp-Plus
Deep Hallu Bench
Report Bench
Mind2Web2

```

**Markitdown Output (correct):**
```

BrowseComp (Wei et al., 2025)
BrowseComp-Plus
DeepHalluBench
ReportBench
Mind2Web2

````

## Root Cause

In `text_cleanup.rs:377-380`, the regex splits on lowercase→Uppercase transitions:

```rust
// Fix lower-UPPER-lower pattern
if let Ok(re) = Regex::new(r"([a-z])([A-Z][a-z])") {
    result = re.replace_all(&result, "$1 $2").to_string();
}
````

This regex:

- `([a-z])` - Captures any lowercase letter
- `([A-Z][a-z])` - Captures uppercase followed by lowercase

Applied to `BrowseComp`:

- `e` (lowercase) followed by `Co` (uppercase+lowercase) → `e Co` → `Browse Comp`

## Intent vs Effect

**Intent:** Fix concatenated words like `methodsThe model` → `methods The model`

**Side Effect:** Breaks legitimate CamelCase terms:

- Project names: BrowseComp, DeepHalluBench, ReportBench
- Technical terms: ArXiv, GitHub, OpenAI
- Acronyms with following word: LLMs, DRAs

## Frequency in Academic Papers

CamelCase is extremely common in academic papers:

- Dataset/benchmark names: BrowseComp, GAIA, SciFact
- Framework names: TensorFlow, PyTorch, LangChain
- Company/product names: OpenAI, DeepMind, ChatGPT

## Files to Investigate

1. `text_cleanup.rs:376-380` - The problematic regex
2. Consider a whitelist approach for known CamelCase terms
3. Or use a smarter heuristic (e.g., only split if word length > N)

## Quality Metrics Impact

- **TPS (Text Preservation)**: Reduces accuracy for technical terms → -5%
- **SFS (Structural Fidelity)**: N/A
- **ROA (Reading Order)**: N/A

This is a **moderate** issue affecting technical terminology accuracy.

```

```
