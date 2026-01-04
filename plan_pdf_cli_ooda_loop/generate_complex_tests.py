#!/usr/bin/env python3
"""
OODA Loop Test Suite Generator - Complex Cases

Generates 20+ test cases using multiple PDF generators:
- pandoc (LaTeX backend)
- weasyprint (CSS-based)
- reportlab (programmatic)
- prince (commercial, high-quality)

Each test case has:
1. Source markdown/HTML
2. Expected gold markdown
3. PDFs from multiple generators
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Optional

# Paths
BASE_DIR = Path(__file__).parent
TEST_SUITE_DIR = BASE_DIR / "00-test-suite-v2"
PDF_DIR = BASE_DIR / "01-generated-pdfs-v2"
GOLD_DIR = BASE_DIR / "02-gold-markdown"

# Ensure directories exist
TEST_SUITE_DIR.mkdir(exist_ok=True)
PDF_DIR.mkdir(exist_ok=True)
GOLD_DIR.mkdir(exist_ok=True)

# Available generators
GENERATORS = {
    "pandoc": lambda html, out: [
        "pandoc",
        "-f",
        "html",
        "-o",
        out,
        "--pdf-engine=pdflatex",
    ],
    "weasyprint": lambda html, out: ["weasyprint", html, out],
    "prince": lambda html, out: ["prince", html, "-o", out],
}


def check_generators():
    """Check which generators are available."""
    available = {}
    for name, _ in GENERATORS.items():
        try:
            if name == "pandoc":
                result = subprocess.run(["pandoc", "--version"], capture_output=True)
            elif name == "weasyprint":
                result = subprocess.run(
                    ["weasyprint", "--version"], capture_output=True
                )
            elif name == "prince":
                result = subprocess.run(["prince", "--version"], capture_output=True)
            available[name] = result.returncode == 0
        except FileNotFoundError:
            available[name] = False
    return available


def md_to_html(md_content: str, title: str = "Test") -> str:
    """Convert markdown to HTML for PDF generation."""
    # Simple markdown to HTML conversion for our test cases
    html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>{title}</title>
    <style>
        body {{ font-family: 'Times New Roman', serif; font-size: 12pt; margin: 2cm; }}
        h1 {{ font-size: 24pt; font-weight: bold; }}
        h2 {{ font-size: 18pt; font-weight: bold; }}
        h3 {{ font-size: 14pt; font-weight: bold; }}
        h4 {{ font-size: 12pt; font-weight: bold; }}
        code {{ font-family: 'Courier New', monospace; background: #f0f0f0; padding: 2px 4px; }}
        pre {{ font-family: 'Courier New', monospace; background: #f0f0f0; padding: 10px; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid black; padding: 8px; text-align: left; }}
        th {{ background: #f0f0f0; font-weight: bold; }}
        blockquote {{ border-left: 3px solid #ccc; padding-left: 10px; margin-left: 0; }}
        .two-column {{ column-count: 2; column-gap: 20px; }}
        img {{ max-width: 100%; }}
    </style>
</head>
<body>
"""
    # Use pandoc for markdown to HTML conversion
    try:
        result = subprocess.run(
            ["pandoc", "-f", "markdown", "-t", "html"],
            input=md_content,
            capture_output=True,
            text=True,
        )
        html += result.stdout
    except:
        html += f"<p>{md_content}</p>"

    html += "\n</body>\n</html>"
    return html


# =============================================================================
# TEST CASES - Complex scenarios to stress-test the converter
# =============================================================================

TEST_CASES = {}

# 07: Multi-level nested lists (deeper than before)
TEST_CASES["07_deep_nested_lists"] = {
    "markdown": """# Deep Nested Lists

## Unordered Deep Nesting

- Level 1 item A
  - Level 2 item A.1
    - Level 3 item A.1.1
      - Level 4 item A.1.1.1
        - Level 5 item A.1.1.1.1
      - Level 4 item A.1.1.2
    - Level 3 item A.1.2
  - Level 2 item A.2
- Level 1 item B

## Ordered Deep Nesting

1. First main point
   1. Sub-point one
      1. Detail level one
         1. Very specific detail
         2. Another specific detail
      2. Detail level two
   2. Sub-point two
2. Second main point

## Mixed Nesting

1. Ordered start
   - Unordered child
     1. Ordered grandchild
        - Unordered great-grandchild
""",
    "gold": """# Deep Nested Lists

## Unordered Deep Nesting

- Level 1 item A
  - Level 2 item A.1
    - Level 3 item A.1.1
      - Level 4 item A.1.1.1
        - Level 5 item A.1.1.1.1
      - Level 4 item A.1.1.2
    - Level 3 item A.1.2
  - Level 2 item A.2
- Level 1 item B

## Ordered Deep Nesting

1. First main point
   1. Sub-point one
      1. Detail level one
         1. Very specific detail
         2. Another specific detail
      2. Detail level two
   2. Sub-point two
2. Second main point

## Mixed Nesting

1. Ordered start
   - Unordered child
     1. Ordered grandchild
        - Unordered great-grandchild
""",
}

# 08: Complex tables with alignment and formatting
TEST_CASES["08_complex_tables"] = {
    "markdown": """# Complex Tables

## Table with Header Row

| Name | Age | City | Occupation |
|------|-----|------|------------|
| Alice | 30 | New York | Engineer |
| Bob | 25 | San Francisco | Designer |
| Charlie | 35 | Chicago | Manager |

## Table with Aligned Columns

| Left | Center | Right |
|:-----|:------:|------:|
| L1 | C1 | R1 |
| Left text | Centered | Right aligned |

## Table with Formatting

| Feature | Status | Notes |
|---------|--------|-------|
| **Bold text** | *Italic* | `Code` |
| Normal | Mixed **bold** and *italic* | Done |
""",
    "gold": """# Complex Tables

## Table with Header Row

| Name | Age | City | Occupation |
|------|-----|------|------------|
| Alice | 30 | New York | Engineer |
| Bob | 25 | San Francisco | Designer |
| Charlie | 35 | Chicago | Manager |

## Table with Aligned Columns

| Left | Center | Right |
|:-----|:------:|------:|
| L1 | C1 | R1 |
| Left text | Centered | Right aligned |

## Table with Formatting

| Feature | Status | Notes |
|---------|--------|-------|
| **Bold text** | *Italic* | `Code` |
| Normal | Mixed **bold** and *italic* | Done |
""",
}

# 09: Block quotes with nesting
TEST_CASES["09_blockquotes"] = {
    "markdown": """# Block Quotes

## Simple Quote

> This is a simple block quote.
> It spans multiple lines.

## Nested Quotes

> First level quote
>
> > Second level nested quote
> > Still in the nested quote
>
> Back to first level

## Quote with Formatting

> **Important:** This quote contains *emphasis* and `code`.
>
> It also has multiple paragraphs within the quote.
""",
    "gold": """# Block Quotes

## Simple Quote

> This is a simple block quote.
> It spans multiple lines.

## Nested Quotes

> First level quote
>
> > Second level nested quote
> > Still in the nested quote
>
> Back to first level

## Quote with Formatting

> **Important:** This quote contains *emphasis* and `code`.
>
> It also has multiple paragraphs within the quote.
""",
}

# 10: Headers at all levels
TEST_CASES["10_all_header_levels"] = {
    "markdown": """# Header Level 1

This is content under H1.

## Header Level 2

This is content under H2.

### Header Level 3

This is content under H3.

#### Header Level 4

This is content under H4.

##### Header Level 5

This is content under H5.

###### Header Level 6

This is content under H6.
""",
    "gold": """# Header Level 1

This is content under H1.

## Header Level 2

This is content under H2.

### Header Level 3

This is content under H3.

#### Header Level 4

This is content under H4.

##### Header Level 5

This is content under H5.

###### Header Level 6

This is content under H6.
""",
}

# 11: Horizontal rules and separators
TEST_CASES["11_horizontal_rules"] = {
    "markdown": """# Document with Separators

First section content.

---

Second section after horizontal rule.

***

Third section after asterisk rule.

___

Fourth section after underscore rule.
""",
    "gold": """# Document with Separators

First section content.

---

Second section after horizontal rule.

---

Third section after asterisk rule.

---

Fourth section after underscore rule.
""",
}

# 12: Links and references
TEST_CASES["12_links"] = {
    "markdown": """# Links and References

## Inline Links

Visit [Google](https://www.google.com) for search.

Check out [GitHub](https://github.com) for code.

## Email Links

Contact us at [email@example.com](mailto:email@example.com).

## URLs

Direct URL: https://example.com/path/to/page

## Link with Title

[Link with title](https://example.com "Example Site")
""",
    "gold": """# Links and References

## Inline Links

Visit [Google](https://www.google.com) for search.

Check out [GitHub](https://github.com) for code.

## Email Links

Contact us at [email@example.com](mailto:email@example.com).

## URLs

Direct URL: https://example.com/path/to/page

## Link with Title

[Link with title](https://example.com "Example Site")
""",
}

# 13: Mixed inline formatting
TEST_CASES["13_mixed_inline_formatting"] = {
    "markdown": """# Mixed Inline Formatting

## Bold and Italic Combinations

This has **bold text** in the middle.

This has *italic text* in the middle.

This has ***bold and italic*** together.

This has **bold with *nested italic* inside**.

## Code with Formatting

Use the `print()` function to output text.

The variable `user_name` should be a string.

## Complex Sentence

The **important** function `calculate()` returns *approximately* 3.14159.
""",
    "gold": """# Mixed Inline Formatting

## Bold and Italic Combinations

This has **bold text** in the middle.

This has *italic text* in the middle.

This has ***bold and italic*** together.

This has **bold with *nested italic* inside**.

## Code with Formatting

Use the `print()` function to output text.

The variable `user_name` should be a string.

## Complex Sentence

The **important** function `calculate()` returns *approximately* 3.14159.
""",
}

# 14: Code blocks with various languages
TEST_CASES["14_code_blocks_multi_lang"] = {
    "markdown": """# Code Blocks

## Python

```python
def hello(name: str) -> str:
    return f"Hello, {name}!"

if __name__ == "__main__":
    print(hello("World"))
```

## JavaScript

```javascript
function greet(name) {
    return `Hello, ${name}!`;
}

console.log(greet("World"));
```

## Rust

```rust
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## SQL

```sql
SELECT name, age
FROM users
WHERE age > 21
ORDER BY name;
```
""",
    "gold": """# Code Blocks

## Python

```python
def hello(name: str) -> str:
    return f"Hello, {name}!"

if __name__ == "__main__":
    print(hello("World"))
```

## JavaScript

```javascript
function greet(name) {
    return `Hello, ${name}!`;
}

console.log(greet("World"));
```

## Rust

```rust
fn main() {
    let name = "World";
    println!("Hello, {}!", name);
}
```

## SQL

```sql
SELECT name, age
FROM users
WHERE age > 21
ORDER BY name;
```
""",
}

# 15: Multi-column layout (two columns)
TEST_CASES["15_two_column_layout"] = {
    "markdown": """# Two Column Layout

This document uses a two-column layout for the main content area.

## Column One Content

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

## Column Two Content

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.
""",
    "gold": """# Two Column Layout

This document uses a two-column layout for the main content area.

## Column One Content

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

## Column Two Content

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.

Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.
""",
}

# 16: Footnotes
TEST_CASES["16_footnotes"] = {
    "markdown": """# Document with Footnotes

This is the main text with a footnote reference[^1].

Another paragraph with another footnote[^2].

And one more with a named footnote[^note].

[^1]: This is the first footnote.
[^2]: This is the second footnote.
[^note]: This is a named footnote with more content.
""",
    "gold": """# Document with Footnotes

This is the main text with a footnote reference[^1].

Another paragraph with another footnote[^2].

And one more with a named footnote[^note].

[^1]: This is the first footnote.
[^2]: This is the second footnote.
[^note]: This is a named footnote with more content.
""",
}

# 17: Definition lists
TEST_CASES["17_definition_lists"] = {
    "markdown": """# Definition Lists

Term 1
:   Definition for term 1

Term 2
:   Definition for term 2
:   Another definition for term 2

Complex Term
:   This is a longer definition that spans multiple lines
    and includes additional formatting like **bold** and *italic*.
""",
    "gold": """# Definition Lists

**Term 1**: Definition for term 1

**Term 2**: Definition for term 2. Another definition for term 2

**Complex Term**: This is a longer definition that spans multiple lines and includes additional formatting like **bold** and *italic*.
""",
}

# 18: Task lists (checkboxes)
TEST_CASES["18_task_lists"] = {
    "markdown": """# Task Lists

## Project Checklist

- [x] Create project structure
- [x] Write initial code
- [ ] Add unit tests
- [ ] Write documentation
- [ ] Deploy to production

## Shopping List

- [x] Milk
- [x] Bread
- [ ] Eggs
- [ ] Butter
""",
    "gold": """# Task Lists

## Project Checklist

- [x] Create project structure
- [x] Write initial code
- [ ] Add unit tests
- [ ] Write documentation
- [ ] Deploy to production

## Shopping List

- [x] Milk
- [x] Bread
- [ ] Eggs
- [ ] Butter
""",
}

# 19: Emoji and special characters
TEST_CASES["19_special_characters"] = {
    "markdown": """# Special Characters

## Symbols

Copyright: ©
Registered: ®
Trademark: ™
Degree: 90°
Plus/Minus: ±5%

## Math Symbols

Alpha: α
Beta: β
Gamma: γ
Pi: π
Sigma: Σ
Delta: Δ

## Arrows

Left: ←
Right: →
Up: ↑
Down: ↓
Double: ⇒

## Currency

Dollar: $100
Euro: €50
Pound: £30
Yen: ¥1000
""",
    "gold": """# Special Characters

## Symbols

Copyright: ©
Registered: ®
Trademark: ™
Degree: 90°
Plus/Minus: ±5%

## Math Symbols

Alpha: α
Beta: β
Gamma: γ
Pi: π
Sigma: Σ
Delta: Δ

## Arrows

Left: ←
Right: →
Up: ↑
Down: ↓
Double: ⇒

## Currency

Dollar: $100
Euro: €50
Pound: £30
Yen: ¥1000
""",
}

# 20: Long document with multiple pages
TEST_CASES["20_multi_page_document"] = {
    "markdown": """# Multi-Page Document

## Introduction

This is a longer document designed to span multiple pages. It tests how the PDF converter handles page breaks and maintains document structure across pages.

## Chapter 1: Getting Started

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

### Section 1.1

More content to fill the page and force a page break.

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

### Section 1.2

Additional content for the document.

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

## Chapter 2: Advanced Topics

This chapter covers more advanced material.

### Section 2.1: Theory

The theoretical foundation is important for understanding the practical applications.

### Section 2.2: Practice

Now we apply the theory to real-world examples.

1. First example
2. Second example
3. Third example

## Conclusion

This concludes our multi-page document test.
""",
    "gold": """# Multi-Page Document

## Introduction

This is a longer document designed to span multiple pages. It tests how the PDF converter handles page breaks and maintains document structure across pages.

## Chapter 1: Getting Started

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

### Section 1.1

More content to fill the page and force a page break.

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.

### Section 1.2

Additional content for the document.

Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

## Chapter 2: Advanced Topics

This chapter covers more advanced material.

### Section 2.1: Theory

The theoretical foundation is important for understanding the practical applications.

### Section 2.2: Practice

Now we apply the theory to real-world examples.

1. First example
2. Second example
3. Third example

## Conclusion

This concludes our multi-page document test.
""",
}

# 21: Math equations (LaTeX)
TEST_CASES["21_math_equations"] = {
    "markdown": """# Mathematical Equations

## Inline Math

The quadratic formula is $x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$.

Einstein's famous equation: $E = mc^2$.

## Display Math

$$
\\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
$$

## Matrix

$$
\\begin{bmatrix}
a & b \\\\
c & d
\\end{bmatrix}
$$
""",
    "gold": """# Mathematical Equations

## Inline Math

The quadratic formula is x = (-b ± √(b² - 4ac)) / 2a.

Einstein's famous equation: E = mc².

## Display Math

∫_{-∞}^{∞} e^{-x²} dx = √π

## Matrix

| a | b |
| c | d |
""",
}

# 22: Strikethrough and underline
TEST_CASES["22_strikethrough"] = {
    "markdown": """# Text Decoration

## Strikethrough

This text has ~~strikethrough~~ words.

The old price was ~~$100~~ now $75.

## Combined Formatting

This is ~~**bold and strikethrough**~~.

This is ~~*italic and strikethrough*~~.
""",
    "gold": """# Text Decoration

## Strikethrough

This text has ~~strikethrough~~ words.

The old price was ~~$100~~ now $75.

## Combined Formatting

This is ~~**bold and strikethrough**~~.

This is ~~*italic and strikethrough*~~.
""",
}

# 23: Subscript and superscript
TEST_CASES["23_sub_superscript"] = {
    "markdown": """# Subscript and Superscript

## Chemical Formulas

Water: H~2~O

Carbon Dioxide: CO~2~

Sulfuric Acid: H~2~SO~4~

## Math Expressions

x^2^ + y^2^ = z^2^

E = mc^2^

## Combined

The formula H~2~O^+^ represents a hydronium ion.
""",
    "gold": """# Subscript and Superscript

## Chemical Formulas

Water: H₂O

Carbon Dioxide: CO₂

Sulfuric Acid: H₂SO₄

## Math Expressions

x² + y² = z²

E = mc²

## Combined

The formula H₂O⁺ represents a hydronium ion.
""",
}

# 24: Academic paper structure
TEST_CASES["24_academic_paper"] = {
    "markdown": """# A Study of PDF Conversion Quality

**Authors:** John Smith, Jane Doe

**Abstract:** This paper examines the quality of PDF to Markdown conversion. We analyze various edge cases and propose improvements.

## 1. Introduction

PDF documents are ubiquitous in academic and professional settings. Converting them to Markdown enables better text processing.

## 2. Methodology

We created a test suite of 30 documents covering:

- Basic text formatting
- Tables and lists
- Images and diagrams
- Mathematical equations

## 3. Results

Our converter achieved 95% accuracy on simple documents and 78% on complex layouts.

| Document Type | Accuracy |
|---------------|----------|
| Simple text | 98% |
| Tables | 85% |
| Multi-column | 72% |

## 4. Discussion

The results demonstrate that layout complexity significantly impacts conversion quality.

## 5. Conclusion

Future work should focus on improving multi-column layout detection.

## References

1. Smith, J. (2024). PDF Processing Techniques.
2. Doe, J. (2023). Markdown Best Practices.
""",
    "gold": """# A Study of PDF Conversion Quality

**Authors:** John Smith, Jane Doe

**Abstract:** This paper examines the quality of PDF to Markdown conversion. We analyze various edge cases and propose improvements.

## 1. Introduction

PDF documents are ubiquitous in academic and professional settings. Converting them to Markdown enables better text processing.

## 2. Methodology

We created a test suite of 30 documents covering:

- Basic text formatting
- Tables and lists
- Images and diagrams
- Mathematical equations

## 3. Results

Our converter achieved 95% accuracy on simple documents and 78% on complex layouts.

| Document Type | Accuracy |
|---------------|----------|
| Simple text | 98% |
| Tables | 85% |
| Multi-column | 72% |

## 4. Discussion

The results demonstrate that layout complexity significantly impacts conversion quality.

## 5. Conclusion

Future work should focus on improving multi-column layout detection.

## References

1. Smith, J. (2024). PDF Processing Techniques.
2. Doe, J. (2023). Markdown Best Practices.
""",
}

# 25: Invoice/Receipt format
TEST_CASES["25_invoice_format"] = {
    "markdown": """# INVOICE

**Invoice Number:** INV-2024-001

**Date:** January 4, 2026

---

**From:**
Acme Corporation
123 Business Street
New York, NY 10001

**To:**
Customer Inc.
456 Client Avenue
Los Angeles, CA 90001

---

## Items

| Description | Quantity | Unit Price | Total |
|-------------|----------|------------|-------|
| Widget A | 10 | $25.00 | $250.00 |
| Widget B | 5 | $50.00 | $250.00 |
| Service Fee | 1 | $100.00 | $100.00 |

---

**Subtotal:** $600.00

**Tax (10%):** $60.00

**Total Due:** $660.00

---

*Payment due within 30 days.*
""",
    "gold": """# INVOICE

**Invoice Number:** INV-2024-001

**Date:** January 4, 2026

---

**From:**
Acme Corporation
123 Business Street
New York, NY 10001

**To:**
Customer Inc.
456 Client Avenue
Los Angeles, CA 90001

---

## Items

| Description | Quantity | Unit Price | Total |
|-------------|----------|------------|-------|
| Widget A | 10 | $25.00 | $250.00 |
| Widget B | 5 | $50.00 | $250.00 |
| Service Fee | 1 | $100.00 | $100.00 |

---

**Subtotal:** $600.00

**Tax (10%):** $60.00

**Total Due:** $660.00

---

*Payment due within 30 days.*
""",
}

# 26: Code with complex indentation
TEST_CASES["26_complex_code_indent"] = {
    "markdown": """# Code Indentation Test

## Python with Nested Structures

```python
class DataProcessor:
    def __init__(self, config: dict):
        self.config = config
        self.results = []
    
    def process(self, data: list) -> list:
        for item in data:
            if item.get("type") == "A":
                result = self._process_type_a(item)
                if result:
                    for sub in result:
                        self.results.append({
                            "source": item,
                            "processed": sub,
                            "metadata": {
                                "timestamp": time.now(),
                                "version": "1.0"
                            }
                        })
        return self.results
```

## YAML Configuration

```yaml
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    certificate: /path/to/cert.pem
    key: /path/to/key.pem
  
database:
  connections:
    - name: primary
      host: db1.example.com
      port: 5432
    - name: replica
      host: db2.example.com
      port: 5432
```
""",
    "gold": """# Code Indentation Test

## Python with Nested Structures

```python
class DataProcessor:
    def __init__(self, config: dict):
        self.config = config
        self.results = []
    
    def process(self, data: list) -> list:
        for item in data:
            if item.get("type") == "A":
                result = self._process_type_a(item)
                if result:
                    for sub in result:
                        self.results.append({
                            "source": item,
                            "processed": sub,
                            "metadata": {
                                "timestamp": time.now(),
                                "version": "1.0"
                            }
                        })
        return self.results
```

## YAML Configuration

```yaml
server:
  host: localhost
  port: 8080
  ssl:
    enabled: true
    certificate: /path/to/cert.pem
    key: /path/to/key.pem
  
database:
  connections:
    - name: primary
      host: db1.example.com
      port: 5432
    - name: replica
      host: db2.example.com
      port: 5432
```
""",
}


def generate_pdf(name: str, html_content: str, generator: str) -> Optional[Path]:
    """Generate a PDF using the specified generator."""
    if generator not in GENERATORS:
        return None

    # Write HTML to temp file
    html_file = PDF_DIR / f"{name}_{generator}.html"
    pdf_file = PDF_DIR / f"{name}_{generator}.pdf"

    with open(html_file, "w") as f:
        f.write(html_content)

    try:
        if generator == "pandoc":
            cmd = [
                "pandoc",
                str(html_file),
                "-o",
                str(pdf_file),
                "--pdf-engine=pdflatex",
            ]
        elif generator == "weasyprint":
            cmd = ["weasyprint", str(html_file), str(pdf_file)]
        elif generator == "prince":
            cmd = ["prince", str(html_file), "-o", str(pdf_file)]
        else:
            return None

        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  ⚠️  {generator} failed: {result.stderr[:100]}")
            return None

        return pdf_file
    except Exception as e:
        print(f"  ⚠️  {generator} error: {e}")
        return None


def main():
    print("=" * 60)
    print("OODA Loop Test Suite Generator - Complex Cases")
    print("=" * 60)

    # Check available generators
    print("\n📋 Checking PDF generators...")
    available = check_generators()
    for name, avail in available.items():
        status = "✅" if avail else "❌"
        print(f"  {status} {name}")

    generators_to_use = [g for g, a in available.items() if a]
    if not generators_to_use:
        print("\n❌ No PDF generators available!")
        sys.exit(1)

    print(f"\n📁 Output directories:")
    print(f"  Test suite: {TEST_SUITE_DIR}")
    print(f"  PDFs: {PDF_DIR}")
    print(f"  Gold: {GOLD_DIR}")

    # Generate test cases
    print(f"\n📝 Generating {len(TEST_CASES)} test cases...")

    for name, data in TEST_CASES.items():
        print(f"\n  {name}:")

        # Save markdown
        md_file = TEST_SUITE_DIR / f"{name}.md"
        with open(md_file, "w") as f:
            f.write(data["markdown"])
        print(f"    ✅ Markdown saved")

        # Save gold
        gold_file = GOLD_DIR / f"{name}.gold.md"
        with open(gold_file, "w") as f:
            f.write(data["gold"])
        print(f"    ✅ Gold saved")

        # Generate HTML
        html_content = md_to_html(data["markdown"], name)

        # Generate PDFs with each generator
        for gen in generators_to_use:
            pdf_path = generate_pdf(name, html_content, gen)
            if pdf_path and pdf_path.exists():
                size = pdf_path.stat().st_size
                print(f"    ✅ {gen}: {size:,} bytes")

    print("\n" + "=" * 60)
    print(f"✅ Generated {len(TEST_CASES)} test cases")
    print(f"   Using {len(generators_to_use)} PDF generators")
    print("=" * 60)


if __name__ == "__main__":
    main()
