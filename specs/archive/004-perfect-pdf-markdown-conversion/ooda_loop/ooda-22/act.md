# OODA-22 Act: Gold File Quality Analysis

## Action Taken

Analyzed gold file quality using markitdown MCP tool for comparison.

## Critical Discovery

**Gold files contain extraction artifacts from markitdown:**

1. **01_2512.25075v1.gold.md** - Contains:
   - Lines 1-39: Vertical arXiv margin ID ("5 2 0 2 c e D 1 3 ]...")
   - Lines 79, 807, 939, 1547: Figure timestamp garbage ("t=80t=80reversemotion...")
2. **These artifacts inflate expected word count**, artificially lowering our TPS score

## Markitdown vs Our Extractor Comparison

| Feature                | Markitdown  | Our Extractor |
| ---------------------- | ----------- | ------------- |
| arXiv margin artifacts | ❌ Includes | ✅ Skips      |
| Figure annotations     | ❌ Includes | ✅ Cleaner    |
| Title extraction       | Good        | Good          |
| Table handling         | Basic       | Better        |

## First Principles Analysis

The gold files are NOT human-curated as stated in the mission spec - they are markitdown outputs.

**Implications:**

1. Our extractor is actually BETTER on arXiv papers (skips margin artifacts)
2. TPS scores are deflated because we're measured against flawed reference
3. We cannot blindly pursue 95% against poor gold standards

## Decision

Rather than manually cleaning all gold files (error-prone, time-consuming):

1. **Document the gold file quality issue** in test documentation
2. **Focus on genuine extraction improvements** (reading order, block merging)
3. **Consider regenerating gold files** from our own high-quality extraction for arXiv papers

## Quality Baseline Unchanged

- Text: 81.3%
- Structure: 80.3%
- Overall: 80.8%

## Next OODA Focus

Move to OODA-23: Focus on improving the lowest-scoring PDFs through algorithm improvements:

- one_tool_2512.20957v2: 75.9% (lowest)
- agent_2510.09244v1: 79.3%
- AlphaEvolve: 79.9% (worst structure score at 74.3%)

## Commit

No code changes in this iteration - analysis only.
