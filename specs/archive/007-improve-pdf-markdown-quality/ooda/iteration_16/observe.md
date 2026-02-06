# OODA Iteration 16 - Observe

## Timestamp

2025-01-25T10:00:00Z

## Observation

After IT15 (bold-to-headers conversion), examining converted PDFs reveals a significant **line break problem** where:

1. **Mid-word breaks**: Words split across lines due to PDF text box boundaries
   - `TCP/IP netw` + `orking` should be `TCP/IP networking`
   - `web` + ` browsers` should stay together as `web browsers`

2. **Hyphenated continuations**: Lines ending with `- ` followed by continuation
   - `All sockets` + `- based` should be `All sockets-based`

3. **Lowercase line starts**: Lines starting with lowercase letters indicate they're continuations
   - `granular ` is continuation of "with many different"
   - `orking is prohibited.` continues from `TCP/IP netw`

## Evidence

Apple-Sandbox-Guide-v1.0.pdf conversion shows:

```
- kSBXProfileNoInternet : TCP/IP netw
orking is prohibited.
- kSBXProfileNoNetwork : All sockets
              - based networking is prohibited.
```

Should be:

```
- kSBXProfileNoInternet : TCP/IP networking is prohibited.
- kSBXProfileNoNetwork : All sockets-based networking is prohibited.
```

## Impact Assessment

- **Affects**: Text coherence, readability, downstream NLP/LLM processing
- **Quality dimension**: Paragraph reconstruction, word integrity
- **Estimated improvement**: Lists 55→70 (joining bullet continuations)

## Root Cause

PDF text extraction preserves the original text box boundaries from the PDF layout, which don't align with logical paragraph/sentence boundaries. Words are broken where they wrapped in the original document.
