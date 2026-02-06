# OODA-09 Orient: Font Name Pattern Analysis

## Root Cause

PDF fonts use various naming conventions:

1. **Standard**: "Arial-Bold", "Times-Italic"
2. **Abbreviated**: "NimbusRomNo9L-ReguItal", "MediItal"
3. **LaTeX**: "SFTI", "CMTI", "CMMI" (Computer Modern)

Our detection only caught (1) and (3), missing (2).

## Font Naming Convention Table

| Font Name              | Bold? | Italic? | Pattern         |
| ---------------------- | ----- | ------- | --------------- |
| Arial-Bold             | ✓     |         | "bold"          |
| Times-Italic           |       | ✓       | "italic"        |
| NimbusRomNo9L-Medi     | ✓     |         | "medi"          |
| NimbusRomNo9L-ReguItal |       | ✓       | "ital"          |
| NimbusRomNo9L-MediItal | ✓     | ✓       | "medi" + "ital" |
| SFTI0900               |       | ✓       | "sfti"          |
| CMMI10                 |       | ✓       | "cmmi"          |

## Why "ital" Pattern is Safe

Adding `lower.contains("ital")` catches all of:

- "italic" (already matched, no harm)
- "ReguItal", "MediItal" (NEW - Nimbus fonts)
- Any other abbreviated italic fonts

The pattern is contained in "italic", so adding it separately just expands coverage without changing existing behavior.

## Why "medi" Should Be Enabled

The previous concern was "over-bolding heading text", but:

1. Headings are detected by font SIZE ratio, not font style
2. Bold formatting is rendered SEPARATELY from header level
3. If the font IS bold (like NimbusRomNo9L-Medi), it SHOULD render as **bold**

The gold standard shows abstract text should be bold - we were missing this.
