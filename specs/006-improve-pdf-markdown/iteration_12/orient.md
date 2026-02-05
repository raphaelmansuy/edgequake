# OODA-12: Orient - Document Constants in block_classifier.rs

## Analysis

### ratio >= 2.0 (heading level 1)

**Purpose**: Classify very large text as H1 (#)
- 2.0x = double body size
- Typical paper titles are 14-16pt on 10pt body = 1.4-1.6x
- 2.0x catches document titles and major sections

**Rationale**: Very large text (2x body) is clearly a major heading.

### ratio >= 1.7 (heading level 2)

**Purpose**: Classify large text as H2 (##)
- 1.7x = 70% larger than body
- Sub-section headings are often 12pt on 10pt = 1.2x
- 1.7x is between subsection (1.5) and title (2.0)

**Rationale**: Mid-size large text is a secondary heading.

### 0.5 uppercase ratio

**Purpose**: Detect all-caps sections like "ABSTRACT"
- 0.5 = 50% uppercase letters
- True all-caps would be 100%, but mixed-case exists
- 50% catches "REFERENCES" with some lowercase

**Rationale**: Lenient threshold for imperfect OCR/extraction.

## Prioritization

1. Heading level ratios (2.0, 1.7) - affect document structure
2. Uppercase ratio (0.5) - affects section detection
