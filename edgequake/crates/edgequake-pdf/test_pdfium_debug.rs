use pdfium_render::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to bind to Pdfium
    let bindings =
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("libs/lib"))?;
    let pdfium = Pdfium::new(bindings);

    // Load the test PDF
    let pdf_bytes = std::fs::read("test-data/001_basic_single_column_text.pdf")?;
    let document = pdfium.load_pdf_from_byte_vec(pdf_bytes, None)?;

    println!("PDF loaded successfully");
    println!("Page count: {}", document.pages().len());

    // Try to iterate pages
    for (i, page) in document.pages().iter().enumerate() {
        println!(
            "Page {}: {}x{}",
            i + 1,
            page.width().value,
            page.height().value
        );

        // Get text
        let text_page = page.text()?;
        let all_text = text_page.all();
        println!("Text length: {} chars", all_text.len());
        println!("Text: {}", all_text);
    }

    Ok(())
}
