#!/usr/bin/env python3
"""
Generate test cases with images and diagrams.

Creates PDFs with:
- SVG diagrams converted to images
- Screenshots of text (OCR test)
- Charts and graphs
- Flowcharts
"""

import io
import os
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

BASE_DIR = Path(__file__).parent
TEST_SUITE_DIR = BASE_DIR / "00-test-suite-v2"
PDF_DIR = BASE_DIR / "01-generated-pdfs-v2"
GOLD_DIR = BASE_DIR / "02-gold-markdown"
ASSETS_DIR = BASE_DIR / "assets"

ASSETS_DIR.mkdir(exist_ok=True)


def create_text_image(text: str, filename: str, width: int = 400, height: int = 100):
    """Create an image with text (for OCR testing)."""
    img = Image.new("RGB", (width, height), color="white")
    draw = ImageDraw.Draw(img)

    # Try to use a system font
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 24)
    except:
        font = ImageFont.load_default()

    # Draw text centered
    draw.text((20, height // 3), text, font=font, fill="black")

    filepath = ASSETS_DIR / filename
    img.save(filepath)
    return filepath


def create_simple_diagram(filename: str):
    """Create a simple flowchart diagram."""
    img = Image.new("RGB", (400, 300), color="white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    except:
        font = ImageFont.load_default()

    # Draw boxes
    draw.rectangle([50, 30, 150, 70], outline="black", width=2)
    draw.text((70, 42), "Start", font=font, fill="black")

    draw.rectangle([50, 120, 150, 160], outline="black", width=2)
    draw.text((60, 132), "Process", font=font, fill="black")

    draw.rectangle([50, 210, 150, 250], outline="black", width=2)
    draw.text((80, 222), "End", font=font, fill="black")

    # Draw arrows
    draw.line([100, 70, 100, 120], fill="black", width=2)
    draw.polygon([(100, 120), (95, 110), (105, 110)], fill="black")

    draw.line([100, 160, 100, 210], fill="black", width=2)
    draw.polygon([(100, 210), (95, 200), (105, 200)], fill="black")

    filepath = ASSETS_DIR / filename
    img.save(filepath)
    return filepath


def create_chart_image(filename: str):
    """Create a simple bar chart."""
    img = Image.new("RGB", (400, 300), color="white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 12)
    except:
        font = ImageFont.load_default()

    # Draw axes
    draw.line([50, 250, 50, 30], fill="black", width=2)  # Y axis
    draw.line([50, 250, 370, 250], fill="black", width=2)  # X axis

    # Draw bars
    bars = [("Q1", 150), ("Q2", 180), ("Q3", 120), ("Q4", 200)]
    colors = ["#4285f4", "#34a853", "#fbbc05", "#ea4335"]

    x = 80
    for (label, height), color in zip(bars, colors):
        draw.rectangle([x, 250 - height, x + 50, 250], fill=color)
        draw.text((x + 15, 255), label, font=font, fill="black")
        x += 80

    # Title
    draw.text((150, 10), "Quarterly Results", font=font, fill="black")

    filepath = ASSETS_DIR / filename
    img.save(filepath)
    return filepath


def create_table_image(filename: str):
    """Create an image of a table (for testing image OCR on tables)."""
    img = Image.new("RGB", (400, 200), color="white")
    draw = ImageDraw.Draw(img)

    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    except:
        font = ImageFont.load_default()

    # Draw table grid
    # Horizontal lines
    for y in [30, 60, 90, 120, 150]:
        draw.line([20, y, 380, y], fill="black", width=1)

    # Vertical lines
    for x in [20, 120, 220, 320, 380]:
        draw.line([x, 30, x, 150], fill="black", width=1)

    # Headers
    headers = ["Name", "Age", "City"]
    x_pos = [40, 140, 240]
    for header, x in zip(headers, x_pos):
        draw.text((x, 38), header, font=font, fill="black")

    # Data
    data = [["Alice", "30", "NYC"], ["Bob", "25", "LA"], ["Carol", "35", "Chicago"]]

    for row_idx, row in enumerate(data):
        y = 68 + row_idx * 30
        for col_idx, cell in enumerate(row):
            draw.text((x_pos[col_idx], y), cell, font=font, fill="black")

    filepath = ASSETS_DIR / filename
    img.save(filepath)
    return filepath


def generate_image_test_cases():
    """Generate test cases with images."""

    print("📷 Generating image assets...")

    # Create images
    text_img = create_text_image(
        "This text is rendered as an image.", "text_as_image.png"
    )
    print(f"  ✅ {text_img}")

    diagram_img = create_simple_diagram("flowchart.png")
    print(f"  ✅ {diagram_img}")

    chart_img = create_chart_image("bar_chart.png")
    print(f"  ✅ {chart_img}")

    table_img = create_table_image("table_image.png")
    print(f"  ✅ {table_img}")

    # Test case 27: Document with text image (OCR test)
    img27_html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>OCR Test - Text as Image</title>
    <style>
        body {{ font-family: 'Times New Roman', serif; font-size: 12pt; margin: 2cm; }}
        h1 {{ font-size: 24pt; }}
        img {{ max-width: 100%; border: 1px solid #ccc; }}
    </style>
</head>
<body>
    <h1>OCR Test - Text as Image</h1>
    <p>The following image contains text that should be extracted via OCR:</p>
    <img src="{text_img.absolute()}" alt="Text rendered as image">
    <p>The text in the image above says: "This text is rendered as an image."</p>
</body>
</html>
"""

    # Test case 28: Document with flowchart
    img28_html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Flowchart Diagram</title>
    <style>
        body {{ font-family: 'Times New Roman', serif; font-size: 12pt; margin: 2cm; }}
        h1 {{ font-size: 24pt; }}
        img {{ max-width: 100%; }}
        .caption {{ font-style: italic; text-align: center; }}
    </style>
</head>
<body>
    <h1>Flowchart Diagram</h1>
    <p>This document contains a flowchart diagram:</p>
    <img src="{diagram_img.absolute()}" alt="Simple flowchart showing Start, Process, End">
    <p class="caption">Figure 1: A simple process flowchart</p>
    <p>The flowchart shows three steps: Start, Process, and End.</p>
</body>
</html>
"""

    # Test case 29: Document with chart
    img29_html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Bar Chart</title>
    <style>
        body {{ font-family: 'Times New Roman', serif; font-size: 12pt; margin: 2cm; }}
        h1 {{ font-size: 24pt; }}
        img {{ max-width: 100%; }}
        .caption {{ font-style: italic; text-align: center; }}
    </style>
</head>
<body>
    <h1>Quarterly Results Chart</h1>
    <p>The following chart shows quarterly performance:</p>
    <img src="{chart_img.absolute()}" alt="Bar chart showing Q1, Q2, Q3, Q4 results">
    <p class="caption">Figure 1: Quarterly Results</p>
    <p>Q4 had the highest performance with 200 units.</p>
</body>
</html>
"""

    # Test case 30: Document with table as image (OCR challenge)
    img30_html = f"""<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Table as Image</title>
    <style>
        body {{ font-family: 'Times New Roman', serif; font-size: 12pt; margin: 2cm; }}
        h1 {{ font-size: 24pt; }}
        img {{ max-width: 100%; }}
    </style>
</head>
<body>
    <h1>Table Rendered as Image</h1>
    <p>This document contains a table rendered as an image:</p>
    <img src="{table_img.absolute()}" alt="Table with Name, Age, City columns">
    <p>The table contains information about Alice (30, NYC), Bob (25, LA), and Carol (35, Chicago).</p>
</body>
</html>
"""

    # Generate PDFs
    test_cases = [
        (
            "27_ocr_text_image",
            img27_html,
            """# OCR Test - Text as Image

The following image contains text that should be extracted via OCR:

![Text rendered as image](text_as_image.png)

*Image text: "This text is rendered as an image."*

The text in the image above says: "This text is rendered as an image."
""",
        ),
        (
            "28_flowchart_diagram",
            img28_html,
            """# Flowchart Diagram

This document contains a flowchart diagram:

![Simple flowchart showing Start, Process, End](flowchart.png)

*Figure 1: A simple process flowchart*

The flowchart shows three steps: Start, Process, and End.
""",
        ),
        (
            "29_bar_chart",
            img29_html,
            """# Quarterly Results Chart

The following chart shows quarterly performance:

![Bar chart showing Q1, Q2, Q3, Q4 results](bar_chart.png)

*Figure 1: Quarterly Results*

Q4 had the highest performance with 200 units.
""",
        ),
        (
            "30_table_as_image",
            img30_html,
            """# Table Rendered as Image

This document contains a table rendered as an image:

![Table with Name, Age, City columns](table_image.png)

| Name | Age | City |
|------|-----|------|
| Alice | 30 | NYC |
| Bob | 25 | LA |
| Carol | 35 | Chicago |

The table contains information about Alice (30, NYC), Bob (25, LA), and Carol (35, Chicago).
""",
        ),
    ]

    print("\n📝 Generating image test cases...")

    for name, html, gold in test_cases:
        print(f"\n  {name}:")

        # Save HTML
        html_file = PDF_DIR / f"{name}.html"
        with open(html_file, "w") as f:
            f.write(html)

        # Save gold
        gold_file = GOLD_DIR / f"{name}.gold.md"
        with open(gold_file, "w") as f:
            f.write(gold)
        print(f"    ✅ Gold saved")

        # Generate PDFs with weasyprint (best for images)
        pdf_file = PDF_DIR / f"{name}_weasyprint.pdf"
        try:
            result = subprocess.run(
                ["weasyprint", str(html_file), str(pdf_file)],
                capture_output=True,
                text=True,
            )
            if result.returncode == 0:
                size = pdf_file.stat().st_size
                print(f"    ✅ weasyprint: {size:,} bytes")
            else:
                print(f"    ⚠️  weasyprint failed: {result.stderr[:100]}")
        except Exception as e:
            print(f"    ⚠️  weasyprint error: {e}")

        # Also try prince
        pdf_file_prince = PDF_DIR / f"{name}_prince.pdf"
        try:
            result = subprocess.run(
                ["prince", str(html_file), "-o", str(pdf_file_prince)],
                capture_output=True,
                text=True,
            )
            if result.returncode == 0:
                size = pdf_file_prince.stat().st_size
                print(f"    ✅ prince: {size:,} bytes")
        except:
            pass


if __name__ == "__main__":
    print("=" * 60)
    print("Image Test Case Generator")
    print("=" * 60)
    generate_image_test_cases()
    print("\n" + "=" * 60)
    print("✅ Image test cases generated")
    print("=" * 60)
