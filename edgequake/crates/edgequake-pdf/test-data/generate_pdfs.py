import os

from reportlab.lib import colors
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.platypus import (
    Frame,
    FrameBreak,
    Image,
    PageBreak,
    PageTemplate,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)


def create_001_simple_text(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Simple Text Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Simple Text Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "This is a simple paragraph of text. It should be extracted as a single block of text in Markdown.",
            styles["Normal"],
        )
    )
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "Another paragraph follows. The extractor should maintain the separation between these two paragraphs.",
            styles["Normal"],
        )
    )
    doc.build(story)


def create_002_headers_and_lists(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Headers and Lists Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Headers and Lists Test", styles["Title"]))
    story.append(Paragraph("Level 1 Header", styles["Heading1"]))
    story.append(Paragraph("Level 2 Header", styles["Heading2"]))
    story.append(Paragraph("Level 3 Header", styles["Heading3"]))
    story.append(Spacer(1, 12))
    story.append(Paragraph("Unordered List:", styles["Normal"]))
    story.append(Paragraph("- Item 1", styles["Normal"]))
    story.append(Paragraph("- Item 2", styles["Normal"]))
    story.append(Paragraph("- Item 3", styles["Normal"]))
    story.append(Spacer(1, 12))
    story.append(Paragraph("Ordered List:", styles["Normal"]))
    story.append(Paragraph("1. First item", styles["Normal"]))
    story.append(Paragraph("2. Second item", styles["Normal"]))
    story.append(Paragraph("3. Third item", styles["Normal"]))
    doc.build(story)


def create_003_two_columns(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Two Column Layout Test"
    styles = getSampleStyleSheet()

    # Define two columns
    frame1 = Frame(
        doc.leftMargin, doc.bottomMargin, doc.width / 2 - 6, doc.height, id="col1"
    )
    frame2 = Frame(
        doc.leftMargin + doc.width / 2 + 6,
        doc.bottomMargin,
        doc.width / 2 - 6,
        doc.height,
        id="col2",
    )

    template = PageTemplate(id="two_columns", frames=[frame1, frame2])
    doc.addPageTemplates([template])

    story = []
    story.append(Paragraph("Two Column Layout Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "This is the first column. The text here should be read completely before moving to the second column if the reading order detection is working correctly.",
            styles["Normal"],
        )
    )
    story.append(
        Paragraph(
            "More text in the first column to ensure it fills some space and tests the vertical flow.",
            styles["Normal"],
        )
    )

    story.append(FrameBreak())  # Move to next frame

    story.append(Paragraph("This is the second column.", styles["Title"]))
    story.append(
        Paragraph(
            "The extractor should detect that this is a separate column and not interleave the lines with the first column.",
            styles["Normal"],
        )
    )
    story.append(
        Paragraph(
            "SOTA extraction requires understanding the spatial layout of the page.",
            styles["Normal"],
        )
    )

    doc.build(story)


def create_004_tables(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Tables Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Tables Test", styles["Title"]))
    story.append(Spacer(1, 12))

    data = [
        ["Header 1", "Header 2", "Header 3"],
        ["Row 1, Col 1", "Row 1, Col 2", "Row 1, Col 3"],
        ["Row 2, Col 1", "Row 2, Col 2", "Row 2, Col 3"],
    ]
    t = Table(data)
    t.setStyle(
        TableStyle(
            [
                ("BACKGROUND", (0, 0), (-1, 0), colors.grey),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.whitesmoke),
                ("ALIGN", (0, 0), (-1, -1), "CENTER"),
                ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
                ("BOTTOMPADDING", (0, 0), (-1, 0), 12),
                ("BACKGROUND", (0, 1), (-1, -1), colors.beige),
                ("GRID", (0, 0), (-1, -1), 1, colors.black),
            ]
        )
    )
    story.append(t)
    doc.build(story)


def create_005_mixed_styles(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Mixed Styles Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Mixed Styles Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "This paragraph contains <b>bold text</b>, <i>italic text</i>, and <u>underlined text</u>.",
            styles["Normal"],
        )
    )
    story.append(
        Paragraph(
            "We also have <b><i>bold and italic</i></b> text combined.",
            styles["Normal"],
        )
    )
    story.append(
        Paragraph(
            "This is a <font face='Courier'>monospace font</font> example.",
            styles["Normal"],
        )
    )
    doc.build(story)


def create_006_images_and_captions(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Images and Captions Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Images and Captions Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(
        Paragraph(
            "Below is an image (placeholder) followed by a caption.", styles["Normal"]
        )
    )
    # Since I don't have a real image easily, I'll use a colored box or just text that looks like a caption
    story.append(Spacer(1, 100))  # Placeholder for image
    story.append(
        Paragraph(
            "Figure 1: This is a caption for the missing image above.", styles["Italic"]
        )
    )
    doc.build(story)


def create_007_nested_lists(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Nested Lists Test"
    styles = getSampleStyleSheet()
    story = []
    story.append(Paragraph("Nested Lists Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(Paragraph("- Level 1 Item A", styles["Normal"]))
    story.append(
        Paragraph("&nbsp;&nbsp;&nbsp;&nbsp;- Level 2 Item A.1", styles["Normal"])
    )
    story.append(
        Paragraph("&nbsp;&nbsp;&nbsp;&nbsp;- Level 2 Item A.2", styles["Normal"])
    )
    story.append(Paragraph("- Level 1 Item B", styles["Normal"]))
    story.append(
        Paragraph("&nbsp;&nbsp;&nbsp;&nbsp;1. Level 2 Ordered 1", styles["Normal"])
    )
    story.append(
        Paragraph("&nbsp;&nbsp;&nbsp;&nbsp;2. Level 2 Ordered 2", styles["Normal"])
    )
    doc.build(story)


def create_008_multi_page(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Multi-page Test"
    styles = getSampleStyleSheet()
    story = []

    for i in range(1, 4):
        story.append(Paragraph(f"Page {i} Content", styles["Title"]))
        story.append(
            Paragraph(
                f"This is some content on page {i}. It should be extracted correctly across page boundaries.",
                styles["Normal"],
            )
        )
        story.append(Spacer(1, 400))
        story.append(Paragraph(f"Footer Page {i}", styles["Normal"]))
        if i < 3:
            story.append(PageBreak())

    doc.build(story)


def create_009_code_blocks(path):
    doc = SimpleDocTemplate(path, pagesize=letter)
    doc.title = "Code Blocks Test"
    styles = getSampleStyleSheet()
    code_style = ParagraphStyle(
        "Code", parent=styles["Normal"], fontName="Courier", fontSize=10, leftIndent=20
    )

    story = []
    story.append(Paragraph("Code Blocks Test", styles["Title"]))
    story.append(Spacer(1, 12))
    story.append(Paragraph("Here is a block of code:", styles["Normal"]))
    story.append(Spacer(1, 6))
    story.append(
        Paragraph(
            'fn main() {<br/>&nbsp;&nbsp;&nbsp;&nbsp;println!("Hello, World!");<br/>}',
            code_style,
        )
    )
    story.append(Spacer(1, 12))
    story.append(Paragraph("End of code block.", styles["Normal"]))
    doc.build(story)


if __name__ == "__main__":
    base_path = "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data"
    create_001_simple_text(os.path.join(base_path, "001_simple_text.pdf"))
    create_002_headers_and_lists(os.path.join(base_path, "002_headers_and_lists.pdf"))
    create_003_two_columns(os.path.join(base_path, "003_two_columns.pdf"))
    create_004_tables(os.path.join(base_path, "004_tables.pdf"))
    create_005_mixed_styles(os.path.join(base_path, "005_mixed_styles.pdf"))
    create_006_images_and_captions(
        os.path.join(base_path, "006_images_and_captions.pdf")
    )
    create_007_nested_lists(os.path.join(base_path, "007_nested_lists.pdf"))
    create_008_multi_page(os.path.join(base_path, "008_multi_page.pdf"))
    create_009_code_blocks(os.path.join(base_path, "009_code_blocks.pdf"))
    print("PDFs generated successfully.")
