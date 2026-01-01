#!/usr/bin/env python3
"""
Generate test PDF documents with increasing complexity for EdgeQuake PDF testing.
"""

import os

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT, TA_RIGHT
from reportlab.lib.pagesizes import A4, letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.pdfgen import canvas
from reportlab.platypus import (
    Image,
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)


def create_001_basic_text():
    """Level 1: Basic single-column text document"""
    filename = "001_basic_single_column_text.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    # Title
    story.append(Paragraph("Basic Text Document", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    # Simple paragraphs
    story.append(
        Paragraph(
            "This is a simple test document with basic text content.", styles["Normal"]
        )
    )
    story.append(Spacer(1, 0.1 * inch))

    story.append(
        Paragraph(
            "The purpose of this document is to test the most basic PDF to Markdown conversion. "
            "It contains only plain text paragraphs without any special formatting.",
            styles["Normal"],
        )
    )
    story.append(Spacer(1, 0.1 * inch))

    story.append(
        Paragraph(
            "A well-functioning PDF converter should be able to extract this text accurately "
            "and preserve the paragraph structure in the output Markdown.",
            styles["Normal"],
        )
    )

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_002_formatted_text():
    """Level 2: Text with basic formatting (bold, italic)"""
    filename = "002_formatted_text_bold_italic.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    # Title
    story.append(Paragraph("Formatted Text Document", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    # Heading levels
    story.append(Paragraph("Main Heading", styles["Heading1"]))
    story.append(Spacer(1, 0.1 * inch))

    story.append(Paragraph("Subheading Level 2", styles["Heading2"]))
    story.append(Spacer(1, 0.1 * inch))

    # Bold and italic text
    story.append(
        Paragraph(
            "This paragraph contains <b>bold text</b> and <i>italic text</i> to test "
            "basic inline formatting detection.",
            styles["Normal"],
        )
    )
    story.append(Spacer(1, 0.1 * inch))

    story.append(
        Paragraph(
            "We can also have <b><i>bold italic text</i></b> and regular text mixed together.",
            styles["Normal"],
        )
    )

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_003_lists():
    """Level 2: Documents with bullet points and numbered lists"""
    filename = "003_lists_bullets_numbered.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()

    # Create list styles
    bullet_style = ParagraphStyle(
        "BulletStyle",
        parent=styles["Normal"],
        leftIndent=20,
        bulletIndent=10,
    )

    story = []

    story.append(Paragraph("Lists and Bullets", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    story.append(Paragraph("Unordered List:", styles["Heading2"]))
    story.append(Spacer(1, 0.1 * inch))

    # Bullet points
    for item in ["First bullet point", "Second bullet point", "Third bullet point"]:
        story.append(Paragraph(f"• {item}", bullet_style))
        story.append(Spacer(1, 0.05 * inch))

    story.append(Spacer(1, 0.2 * inch))
    story.append(Paragraph("Ordered List:", styles["Heading2"]))
    story.append(Spacer(1, 0.1 * inch))

    # Numbered list
    for i, item in enumerate(
        ["First numbered item", "Second numbered item", "Third numbered item"], 1
    ):
        story.append(Paragraph(f"{i}. {item}", bullet_style))
        story.append(Spacer(1, 0.05 * inch))

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_004_simple_table():
    """Level 4: Simple table structure"""
    filename = "004_simple_table_2x3.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    story.append(Paragraph("Simple Table", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    story.append(
        Paragraph(
            "This document contains a simple 2-column by 3-row table:", styles["Normal"]
        )
    )
    story.append(Spacer(1, 0.2 * inch))

    # Simple table
    data = [["Name", "Age"], ["Alice", "25"], ["Bob", "30"], ["Charlie", "35"]]

    table = Table(data, colWidths=[2 * inch, 2 * inch])
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), colors.grey),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
                ("ALIGN", (0, 0), (-1, -1), "CENTER"),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("FONTSIZE", (0, 0), (-1, 0), 12),
                ("BOTTOMPADDING", (0, 0), (-1, 0), 12),
                ("BACKGROUND", (0, 1), (-1, -1), colors.beige),
                ("GRID", (0, 0), (-1, -1), 1, colors.black),
            ]
        )
    )

    story.append(table)

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_005_complex_table():
    """Level 4: Complex table with merged cells and formatting"""
    filename = "005_complex_table_merged_cells.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    story.append(Paragraph("Complex Table with Merged Cells", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    # Complex table
    data = [
        ["Product", "Q1", "Q2", "Q3", "Q4", "Total"],
        ["Widget A", "100", "120", "150", "180", "550"],
        ["Widget B", "80", "90", "110", "130", "410"],
        ["Widget C", "60", "75", "85", "95", "315"],
        ["Total", "240", "285", "345", "405", "1275"],
    ]

    table = Table(
        data, colWidths=[1.5 * inch, 1 * inch, 1 * inch, 1 * inch, 1 * inch, 1 * inch]
    )
    table.setStyle(
        TableStyle(
            [
                # Header row
                ("BACKGROUND", (0, 0), (-1, 0), colors.darkblue),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
                ("ALIGN", (0, 0), (-1, -1), "CENTER"),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("FONTSIZE", (0, 0), (-1, 0), 12),
                ("BOTTOMPADDING", (0, 0), (-1, 0), 12),
                # Data rows
                ("BACKGROUND", (0, 1), (-1, -2), colors.lightblue),
                ("GRID", (0, 0), (-1, -1), 1, colors.black),
                # Total row
                ("BACKGROUND", (0, -1), (-1, -1), colors.grey),
                ("FONTNAME", (0, -1), (-1, -1), "Helvetica-Bold"),
            ]
        )
    )

    story.append(table)

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_006_multi_column():
    """Level 3: Multi-column layout"""
    filename = "006_multi_column_layout.pdf"

    c = canvas.Canvas(filename, pagesize=letter)
    width, height = letter

    # Title
    c.setFont("Helvetica-Bold", 18)
    c.drawString(1 * inch, height - 1 * inch, "Multi-Column Layout")

    # Two columns
    col_width = 2.5 * inch
    col1_x = 1 * inch
    col2_x = 4.5 * inch
    y_start = height - 1.5 * inch

    c.setFont("Helvetica", 10)

    # Column 1
    text1 = [
        "This is the left column of text.",
        "It should be extracted first",
        "according to reading order.",
        "",
        "The PDF converter must detect",
        "that this is a two-column layout",
        "and process it correctly.",
    ]

    y = y_start
    for line in text1:
        c.drawString(col1_x, y, line)
        y -= 15

    # Column 2
    text2 = [
        "This is the right column of text.",
        "It should be extracted after",
        "the left column is complete.",
        "",
        "Proper column detection is crucial",
        "for maintaining document structure",
        "in the markdown output.",
    ]

    y = y_start
    for line in text2:
        c.drawString(col2_x, y, line)
        y -= 15

    c.save()
    print(f"✅ Created {filename}")
    return filename


def create_007_mixed_content():
    """Level 6: Mixed content (text, lists, tables)"""
    filename = "007_mixed_content_complex.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    # Title
    story.append(Paragraph("Mixed Content Document", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))

    # Introduction
    story.append(Paragraph("Introduction", styles["Heading1"]))
    story.append(
        Paragraph(
            "This document contains a realistic mix of different content types that "
            "commonly appear in real-world documents.",
            styles["Normal"],
        )
    )
    story.append(Spacer(1, 0.2 * inch))

    # Section with list
    story.append(Paragraph("Key Features", styles["Heading2"]))
    bullet_style = ParagraphStyle("Bullet", parent=styles["Normal"], leftIndent=20)
    for item in ["Text paragraphs", "Bullet lists", "Tables", "Multiple sections"]:
        story.append(Paragraph(f"• {item}", bullet_style))
        story.append(Spacer(1, 0.05 * inch))

    story.append(Spacer(1, 0.2 * inch))

    # Table
    story.append(Paragraph("Performance Metrics", styles["Heading2"]))
    story.append(Spacer(1, 0.1 * inch))

    data = [
        ["Metric", "Value"],
        ["Accuracy", "95%"],
        ["Speed", "1000 pages/min"],
        ["Quality", "Excellent"],
    ]

    table = Table(data, colWidths=[2.5 * inch, 2 * inch])
    table.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), colors.grey),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
                ("ALIGN", (0, 0), (-1, -1), "LEFT"),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("GRID", (0, 0), (-1, -1), 1, colors.black),
                ("BACKGROUND", (0, 1), (-1, -1), colors.lightgrey),
            ]
        )
    )

    story.append(table)
    story.append(Spacer(1, 0.2 * inch))

    # Conclusion
    story.append(Paragraph("Conclusion", styles["Heading2"]))
    story.append(
        Paragraph(
            "A SOTA PDF converter should handle all these content types seamlessly "
            "and produce clean, well-structured Markdown output.",
            styles["Normal"],
        )
    )

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_008_multi_page():
    """Level 6: Multi-page document"""
    filename = "008_multi_page_5_pages.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []

    for page_num in range(1, 6):
        story.append(Paragraph(f"Page {page_num}", styles["Title"]))
        story.append(Spacer(1, 0.2 * inch))

        story.append(
            Paragraph(
                f"This is the content of page {page_num}. A multi-page document tests "
                f"whether the PDF converter can process multiple pages correctly and "
                f"maintain page boundaries or merge content appropriately.",
                styles["Normal"],
            )
        )
        story.append(Spacer(1, 0.2 * inch))

        story.append(
            Paragraph(
                f"Each page should be extracted sequentially, and the page number tracking "
                f"(if enabled) should correctly identify this as page {page_num}.",
                styles["Normal"],
            )
        )

        if page_num < 5:
            story.append(PageBreak())

    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_readme(test_files):
    """Create README documenting all test cases"""
    readme_content = """# EdgeQuake PDF Test Suite

## Overview
This directory contains test PDF documents with increasing complexity levels to validate the EdgeQuake PDF to Markdown conversion tool.

## Test Documents

### Level 1: Basic Text
- **001_basic_single_column_text.pdf**
  - Single column, plain text only
  - No formatting or special elements
  - Expected output: Clean paragraphs in Markdown

### Level 2: Formatted Text
- **002_formatted_text_bold_italic.pdf**
  - Headings (H1, H2)
  - Bold and italic inline formatting
  - Expected output: Proper Markdown formatting (* for italic, ** for bold)

- **003_lists_bullets_numbered.pdf**
  - Unordered bullet lists
  - Ordered numbered lists
  - Expected output: Proper list syntax (-, 1., 2., etc.)

### Level 3: Structure
- **006_multi_column_layout.pdf**
  - Two-column layout
  - Tests column detection and reading order
  - Expected output: Left column first, then right column

### Level 4: Tables
- **004_simple_table_2x3.pdf**
  - Simple 2-column table with header
  - 3 data rows
  - Expected output: Markdown table syntax with alignment

- **005_complex_table_merged_cells.pdf**
  - Multi-column table (6 columns)
  - Multiple rows with formatting
  - Total row with special styling
  - Expected output: Complete table in Markdown

### Level 6: Mixed Content
- **007_mixed_content_complex.pdf**
  - Realistic mix: text + lists + tables
  - Multiple sections with headings
  - Tests integration of all features
  - Expected output: Well-structured Markdown with all elements

- **008_multi_page_5_pages.pdf**
  - 5-page document
  - Tests page boundary handling
  - Tests --page-numbers flag
  - Expected output: Continuous or page-delimited Markdown

## Testing Protocol (ODAA Loop)

For each test document:

1. **OBSERVE**: Examine the input PDF (use `info` command)
2. **ORIENT**: Define expected Markdown output
3. **DECIDE**: Run conversion, identify issues
4. **ACT**: Fix code if needed
5. **ASSESS**: Verify improvement, iterate

## Test Commands

```bash
# Get PDF info
cargo run --bin edgequake-pdf -- info -i test-data/001_basic_single_column_text.pdf

# Convert to markdown (default output)
cargo run --bin edgequake-pdf -- convert -i test-data/001_basic_single_column_text.pdf

# Convert with custom output
cargo run --bin edgequake-pdf -- convert -i test-data/001_basic_single_column_text.pdf -o output/001.md

# Convert with page numbers
cargo run --bin edgequake-pdf -- convert -i test-data/008_multi_page_5_pages.pdf --page-numbers

# Convert first 3 pages only
cargo run --bin edgequake-pdf -- convert -i test-data/008_multi_page_5_pages.pdf --max-pages 3

# Vision mode (if/when implemented with real LLM)
cargo run --bin edgequake-pdf -- convert -i test-data/007_mixed_content_complex.pdf --vision
```

## Success Criteria (SOTA)

A SOTA PDF converter should:

✅ Extract text accurately (100% for clean PDFs)
✅ Preserve document structure (headings, paragraphs)
✅ Detect and format lists correctly
✅ Handle tables (simple and complex)
✅ Maintain reading order (single/multi-column)
✅ Process multi-page documents efficiently
✅ Generate clean, valid Markdown
✅ Handle edge cases gracefully

## Current Status

See `scratchpad_raw_log.md` for detailed test results and iteration log.
"""

    with open("README.md", "w") as f:
        f.write(readme_content)

    print("✅ Created README.md")


def main():
    """Generate all test PDFs"""
    print("🚀 Generating EdgeQuake PDF test suite...\n")

    test_files = []

    test_files.append(create_001_basic_text())
    test_files.append(create_002_formatted_text())
    test_files.append(create_003_lists())
    test_files.append(create_004_simple_table())
    test_files.append(create_005_complex_table())
    test_files.append(create_006_multi_column())
    test_files.append(create_007_mixed_content())
    test_files.append(create_008_multi_page())

    create_readme(test_files)

    print(f"\n✅ Successfully generated {len(test_files)} test PDFs")
    print("📋 See README.md for test documentation")
    print("📝 Use scratchpad_raw_log.md to document test results")


if __name__ == "__main__":
    main()
