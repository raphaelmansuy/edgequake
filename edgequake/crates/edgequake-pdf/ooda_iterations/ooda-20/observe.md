# OODA-20: Footnote Marker Cleanup - OBSERVE

## Baseline Metrics

- Overall Quality: 86.5%
- Text Preservation: 85.7%
- Structural Fidelity: 87.2%

## Observation

In the `agent_2510.09244v1.mdf.gen` output, footnote markers appear at the start of text:

```
⋆ This paper is based on a seminar technical report from the course Trends in Au
```

The gold file expects clean text without the marker:

```
This paper is based on...
```

## Footnote Markers Found

- ⋆ (U+22C6, SIX POINTED BLACK STAR) - common in arXiv papers
- † (dagger)
- ‡ (double dagger)
- § (section sign)
- ¶ (pilcrow)

## Root Cause

PDF embeds footnote reference symbols as visible text that gets extracted along with body content.

## Impact Assessment

- Affects few words per document
- Impact on word-level quality score: minimal (~0.1%)
- But affects readability and markdown cleanliness
