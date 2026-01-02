# Example PDF Conversion Test Document

## Overview

This is an example PDF to Markdown conversion test document demonstrating all major formatting features that the validator will check.

## Text Formatting

This paragraph demonstrates various text formatting options:

- **Bold text** appears with double asterisks
- _Italic text_ appears with single asterisks
- **_Bold and italic_** combined with triple asterisks

The validator measures how accurately these formats are preserved during PDF to Markdown conversion.

## Headings

Headings are a critical formatting element. This document includes:

### Level 3 Heading

#### Level 4 Heading

The validator checks that heading levels are detected correctly and correspond to font sizes in the original PDF.

## Lists

### Bullet Points

- First item in the list
- Second item with more text
  - Nested bullet point
  - Another nested point
- Third item at top level

### Numbered List

1. First numbered item
2. Second numbered item
   1. Nested numbered item
   2. Another nested number
3. Third top-level item

## Tables

Tables are a critical component. This is a simple 2x3 table:

| Header 1 | Header 2 | Header 3 |
| -------- | -------- | -------- |
| Cell 1   | Cell 2   | Cell 3   |
| Cell 4   | Cell 5   | Cell 6   |

And here's a more complex table with longer content:

| Name    | Description                     | Score |
| ------- | ------------------------------- | ----- |
| Alice   | High performance implementation | 95    |
| Bob     | Standard approach               | 82    |
| Charlie | Experimental variant            | 78    |

## Code Blocks

```python
def hello_world():
    """A simple greeting function."""
    print("Hello, World!")
    return True
```

And a JavaScript example:

```javascript
const greet = (name) => {
  console.log(`Hello, ${name}!`);
};
```

## Mixed Content

Paragraphs can contain **bold text mixed with _italic text_** in the same sentence. The validator ensures these nested formats are preserved correctly.

This paragraph has inline code (`const x = 5`) followed by more text with **bold** and _italic_ combinations.

## Conclusion

This test document covers the main formatting features that a PDF to Markdown converter must handle:

1. **Text styles** (bold, italic, bold-italic)
2. **Heading levels** (H1 through H4 shown here)
3. **Lists** (bullet and numbered, with nesting)
4. **Tables** (simple and complex layouts)
5. **Code blocks** (with language specification)
6. **Mixed formatting** (styles combined in single lines)

The validator computes F1 scores for each dimension and produces a composite score combining all metrics.
