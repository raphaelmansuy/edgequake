#!/usr/bin/env python3
"""
Generate comprehensive test PDF suite for SOTA validation.
Naming convention: NNN_test_name_description.pdf
"""

from reportlab.lib import colors
from reportlab.lib.pagesizes import letter, A4
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib.units import inch
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, Image, KeepTogether, ListFlowable, ListItem
)
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT, TA_JUSTIFY
from reportlab.pdfgen import canvas
import os


def create_013_nested_lists():
    """Test nested list structures"""
    filename = "013_nested_lists_deep.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    story.append(Paragraph("Nested Lists Test", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    # Create nested list manually with indentation
    story.append(Paragraph("1. First level item 1", styles["Normal"]))
    story.append(Paragraph("   a. Second level item 1a", styles["Normal"]))
    story.append(Paragraph("   b. Second level item 1b", styles["Normal"]))
    story.append(Paragraph("      i. Third level item 1b-i", styles["Normal"]))
    story.append(Paragraph("      ii. Third level item 1b-ii", styles["Normal"]))
    story.append(Paragraph("2. First level item 2", styles["Normal"]))
    story.append(Paragraph("   - Second level bullet", styles["Normal"]))
    story.append(Paragraph("   - Another bullet", styles["Normal"]))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_014_complex_table_spanning():
    """Test table with row and column spanning"""
    filename = "014_table_spanning_cells.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    story.append(Paragraph("Table with Spanning Cells", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    # Complex table with merged cells
    data = [
        ['Product', 'Q1', 'Q2', 'Q3', 'Q4', 'Total'],
        ['Widgets', '100', '150', '120', '180', '550'],
        ['Gadgets', '200', '180', '220', '240', '840'],
        ['Tools', '150', '160', '140', '170', '620'],
        ['Total', '450', '490', '480', '590', '2010']
    ]
    
    table = Table(data, colWidths=[1.5*inch, inch, inch, inch, inch, inch])
    table.setStyle(TableStyle([
        ('BACKGROUND', (0, 0), (-1, 0), colors.HexColor('#4A90E2')),
        ('TEXTCOLOR', (0, 0), (-1, 0), colors.whitesmoke),
        ('ALIGN', (0, 0), (-1, -1), 'CENTER'),
        ('FONTNAME', (0, 0), (-1, 0), 'Helvetica-Bold'),
        ('FONTSIZE', (0, 0), (-1, 0), 12),
        ('BOTTOMPADDING', (0, 0), (-1, 0), 12),
        ('BACKGROUND', (0, -1), (-1, -1), colors.HexColor('#E8F4F8')),
        ('FONTNAME', (0, -1), (-1, -1), 'Helvetica-Bold'),
        ('GRID', (0, 0), (-1, -1), 1, colors.black),
        ('ROWBACKGROUNDS', (0, 1), (-1, -2), [colors.white, colors.HexColor('#F5F5F5')]),
    ]))
    
    story.append(table)
    story.append(Spacer(1, 0.3 * inch))
    story.append(Paragraph("Expected: All cells should be extracted accurately with proper alignment.", styles["Normal"]))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_015_superscript_subscript():
    """Test superscript and subscript"""
    filename = "015_superscript_subscript.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    story.append(Paragraph("Superscript and Subscript Test", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    story.append(Paragraph("Mathematical notation: E = mc<super>2</super>", styles["Normal"]))
    story.append(Paragraph("Chemical formula: H<sub>2</sub>O is water", styles["Normal"]))
    story.append(Paragraph("Footnotes: This has a footnote<super>1</super>", styles["Normal"]))
    story.append(Paragraph("Array indexing: x<sub>i</sub> where i goes from 1 to n", styles["Normal"]))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_016_mixed_fonts():
    """Test multiple fonts and sizes"""
    filename = "016_mixed_fonts_sizes.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    # Create custom styles with different fonts
    code_style = ParagraphStyle('Code',
        parent=styles['Normal'],
        fontName='Courier',
        fontSize=10,
        leading=12
    )
    
    large_style = ParagraphStyle('Large',
        parent=styles['Normal'],
        fontSize=18,
        leading=22
    )
    
    small_style = ParagraphStyle('Small',
        parent=styles['Normal'],
        fontSize=8,
        leading=10
    )
    
    story.append(Paragraph("Mixed Fonts and Sizes", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    story.append(Paragraph("Normal text in Helvetica.", styles["Normal"]))
    story.append(Paragraph("def hello_world():\n    print('Hello, World!')", code_style))
    story.append(Paragraph("LARGE TEXT FOR EMPHASIS", large_style))
    story.append(Paragraph("Small fine print text", small_style))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_017_three_columns():
    """Test three-column layout"""
    filename = "017_three_columns.pdf"
    c = canvas.Canvas(filename, pagesize=letter)
    width, height = letter
    
    c.setFont("Helvetica-Bold", 16)
    c.drawCentredString(width/2, height - 50, "Three Column Layout Test")
    
    col_width = width / 3 - 60
    margin = 40
    top = height - 100
    
    # Column 1
    c.setFont("Helvetica-Bold", 12)
    c.drawString(margin, top, "Column 1")
    c.setFont("Helvetica", 10)
    text_col1 = "This is the first column. It contains text that should be read first. More content here to fill the column."
    text = c.beginText(margin, top - 20)
    text.setFont("Helvetica", 10)
    for line in wrap_text(text_col1, 30):
        text.textLine(line)
    c.drawText(text)
    
    # Column 2
    col2_x = margin + col_width + 40
    c.setFont("Helvetica-Bold", 12)
    c.drawString(col2_x, top, "Column 2")
    c.setFont("Helvetica", 10)
    text_col2 = "This is the second column. It should be read after column 1. Content continues here."
    text = c.beginText(col2_x, top - 20)
    text.setFont("Helvetica", 10)
    for line in wrap_text(text_col2, 30):
        text.textLine(line)
    c.drawText(text)
    
    # Column 3
    col3_x = margin + 2 * (col_width + 40)
    c.setFont("Helvetica-Bold", 12)
    c.drawString(col3_x, top, "Column 3")
    c.setFont("Helvetica", 10)
    text_col3 = "This is the third column. It should be read last. Final content here."
    text = c.beginText(col3_x, top - 20)
    text.setFont("Helvetica", 10)
    for line in wrap_text(text_col3, 30):
        text.textLine(line)
    c.drawText(text)
    
    c.showPage()
    c.save()
    print(f"✅ Created {filename}")
    return filename


def wrap_text(text, width):
    """Simple text wrapping"""
    words = text.split()
    lines = []
    current_line = []
    current_length = 0
    
    for word in words:
        if current_length + len(word) + 1 <= width:
            current_line.append(word)
            current_length += len(word) + 1
        else:
            lines.append(' '.join(current_line))
            current_line = [word]
            current_length = len(word) + 1
    
    if current_line:
        lines.append(' '.join(current_line))
    
    return lines


def create_018_table_with_headers():
    """Test complex table with multi-level headers"""
    filename = "018_table_multiheader.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    story.append(Paragraph("Complex Table with Headers", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    # Table with grouped headers
    data = [
        ['', 'Sales', '', 'Costs', ''],
        ['Region', 'Q1', 'Q2', 'Q1', 'Q2'],
        ['North', '100', '120', '80', '90'],
        ['South', '150', '160', '100', '110'],
        ['East', '130', '140', '95', '105'],
        ['West', '110', '125', '85', '95']
    ]
    
    table = Table(data, colWidths=[1.2*inch, 1*inch, 1*inch, 1*inch, 1*inch])
    table.setStyle(TableStyle([
        ('SPAN', (1, 0), (2, 0)),  # Sales spans 2 columns
        ('SPAN', (3, 0), (4, 0)),  # Costs spans 2 columns
        ('BACKGROUND', (0, 0), (-1, 1), colors.grey),
        ('TEXTCOLOR', (0, 0), (-1, 1), colors.whitesmoke),
        ('ALIGN', (0, 0), (-1, -1), 'CENTER'),
        ('FONTNAME', (0, 0), (-1, 1), 'Helvetica-Bold'),
        ('GRID', (0, 0), (-1, -1), 1, colors.black),
        ('ROWBACKGROUNDS', (0, 2), (-1, -1), [colors.white, colors.lightgrey]),
    ]))
    
    story.append(table)
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_019_footnotes():
    """Test footnotes and references"""
    filename = "019_footnotes_references.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    footnote_style = ParagraphStyle('Footnote',
        parent=styles['Normal'],
        fontSize=8,
        leading=10
    )
    
    story.append(Paragraph("Document with Footnotes", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    story.append(Paragraph("This is a statement that needs a citation<super>1</super>.", styles["Normal"]))
    story.append(Paragraph("Another fact with a reference<super>2</super>.", styles["Normal"]))
    story.append(Paragraph("Final point with multiple citations<super>3,4</super>.", styles["Normal"]))
    
    story.append(Spacer(1, 0.5 * inch))
    story.append(Paragraph("─" * 50, footnote_style))
    story.append(Paragraph("<super>1</super> First reference: Smith et al. (2020)", footnote_style))
    story.append(Paragraph("<super>2</super> Second reference: Jones (2019)", footnote_style))
    story.append(Paragraph("<super>3</super> Third reference: Brown (2021)", footnote_style))
    story.append(Paragraph("<super>4</super> Fourth reference: Davis (2022)", footnote_style))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def create_020_unicode_special_chars():
    """Test Unicode and special characters"""
    filename = "020_unicode_special_chars.pdf"
    doc = SimpleDocTemplate(filename, pagesize=letter)
    styles = getSampleStyleSheet()
    story = []
    
    story.append(Paragraph("Unicode and Special Characters", styles["Title"]))
    story.append(Spacer(1, 0.2 * inch))
    
    story.append(Paragraph("Mathematical symbols: ∑ ∫ ∂ √ ∞ ≈ ≠ ≤ ≥", styles["Normal"]))
    story.append(Paragraph("Greek letters: α β γ δ ε θ λ μ π σ ω", styles["Normal"]))
    story.append(Paragraph("Currencies: $ € £ ¥ ₹", styles["Normal"]))
    story.append(Paragraph("Arrows: → ← ↑ ↓ ↔ ⇒ ⇐", styles["Normal"]))
    story.append(Paragraph("Bullets and symbols: • ○ ■ □ ★ ☆ ✓ ✗", styles["Normal"]))
    story.append(Paragraph("Fractions: ½ ⅓ ¼ ¾", styles["Normal"]))
    
    doc.build(story)
    print(f"✅ Created {filename}")
    return filename


def main():
    """Generate all comprehensive test PDFs"""
    print("\n" + "="*50)
    print("Generating Comprehensive Test PDF Suite")
    print("="*50 + "\n")
    
    tests = [
        create_013_nested_lists,
        create_014_complex_table_spanning,
        create_015_superscript_subscript,
        create_016_mixed_fonts,
        create_017_three_columns,
        create_018_table_with_headers,
        create_019_footnotes,
        create_020_unicode_special_chars,
    ]
    
    for test_func in tests:
        try:
            test_func()
        except Exception as e:
            print(f"❌ Error in {test_func.__name__}: {e}")
    
    print("\n" + "="*50)
    print("Test PDF Generation Complete!")
    print("="*50 + "\n")


if __name__ == "__main__":
    main()
