//! Count Tj vs TJ operators on page 1

use lopdf::content::Content;
use lopdf::Document;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    println!("Loading PDF: {}", pdf_path.display());

    let doc = Document::load(pdf_path).expect("Failed to load PDF");

    // Get first page
    let page_ids = doc.get_pages();
    if let Some((_, &page_id)) = page_ids.iter().next() {
        if let Ok(content_bytes) = doc.get_page_content(page_id) {
            let content = Content::decode(&content_bytes).expect("Failed to decode content");

            let mut tj_count = 0;
            let mut big_tj_count = 0;
            let mut other_ops = std::collections::HashMap::new();

            for op in &content.operations {
                match op.operator.as_str() {
                    "Tj" => tj_count += 1,
                    "TJ" => big_tj_count += 1,
                    _ => {
                        *other_ops.entry(op.operator.clone()).or_insert(0) += 1;
                    }
                }
            }

            println!("\nOperator counts:");
            println!("  Tj: {}", tj_count);
            println!("  TJ: {}", big_tj_count);
            println!("\nOther text-related ops:");
            for op in ["BT", "ET", "Tm", "Td", "TD", "T*", "'", "Tf"].iter() {
                if let Some(count) = other_ops.get(*op) {
                    println!("  {}: {}", op, count);
                }
            }
        }
    }
}
