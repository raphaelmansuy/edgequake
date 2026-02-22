#!/usr/bin/env python3
"""
Create a simple three-column test PDF with clear spacing.
"""

from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import letter

def create_simple_three_columns():
    filename = "021_simple_three_columns.pdf"
    c = canvas.Canvas(filename, pagesize=letter)
    width, height = letter
    
    # Title spanning all columns
    c.setFont("Helvetica-Bold", 18)
    c.drawCentredString(width/2, height - 60, "Three Column Layout")
    
    # Define three equal columns with clear gaps
    col_width = (width - 120) / 3  # 40px margins, 20px gaps between columns
    gap = 20
    margin = 40
    top = height - 120
    
    # Column 1
    c.setFont("Helvetica-Bold", 14)
    c.drawString(margin, top, "Column 1")
    c.setFont("Helvetica", 11)
    c.drawString(margin, top - 20, "First column text.")
    c.drawString(margin, top - 35, "More content in")
    c.drawString(margin, top - 50, "column one.")
    
    # Column 2
    x2 = margin + col_width + gap
    c.setFont("Helvetica-Bold", 14)
    c.drawString(x2, top, "Column 2")
    c.setFont("Helvetica", 11)
    c.drawString(x2, top - 20, "Second column text.")
    c.drawString(x2, top - 35, "More content in")
    c.drawString(x2, top - 50, "column two.")
    
    # Column 3
    x3 = margin + 2 * (col_width + gap)
    c.setFont("Helvetica-Bold", 14)
    c.drawString(x3, top, "Column 3")
    c.setFont("Helvetica", 11)
    c.drawString(x3, top - 20, "Third column text.")
    c.drawString(x3, top - 35, "More content in")
    c.drawString(x3, top - 50, "column three.")
    
    c.showPage()
    c.save()
    print(f"✅ Created {filename}")
    return filename

if __name__ == "__main__":
    create_simple_three_columns()
