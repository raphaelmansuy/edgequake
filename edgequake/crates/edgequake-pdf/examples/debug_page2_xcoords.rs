//! Debug example to dump page 2 X coordinates for column analysis
//! Uses raw lopdf to get element coordinates BEFORE processing

use lopdf::Document;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let pdf_path = args.get(1).map(|s| s.as_str()).unwrap_or(
        "/Users/raphaelmansuy/Github/03-working/edgequake/zz_test_docs/agentfail_2601.22984v1.pdf",
    );

    println!("=== Page 2 Raw X Coordinate Analysis ===\n");
    println!("PDF: {}\n", pdf_path);

    let doc = Document::load(pdf_path)?;
    let pages = doc.get_pages();
    let page_ids: Vec<_> = pages.values().cloned().collect();

    if page_ids.len() < 2 {
        println!("PDF has fewer than 2 pages");
        return Ok(());
    }

    // Get page 2
    let page_id = page_ids[1];

    // Get page width
    let page_dict = doc.get_dictionary(page_id)?;
    let page_width = if let Ok(lopdf::Object::Array(arr)) = page_dict.get(b"MediaBox") {
        if arr.len() >= 4 {
            get_number(&arr[2]).unwrap_or(612.0)
        } else {
            612.0
        }
    } else {
        612.0
    };

    println!("Page 2: width={:.1}pt\n", page_width);
    println!("Expected column layout for arXiv papers:");
    println!("  Left column:  x = 55 to ~280");
    println!("  Gutter:       x = ~280 to ~310 (gap)");
    println!("  Right column: x = ~310 to 555");
    println!();

    // Parse content stream to get Tm/Td positions
    let content_bytes = get_page_content(&doc, page_id)?;
    let operations = lopdf::content::Content::decode(&content_bytes)?;

    // Track text matrix positions
    let mut x_positions: Vec<f32> = Vec::new();
    let mut current_tm_x: f32 = 0.0;
    let mut current_td_x: f32 = 0.0;

    for op in &operations.operations {
        match op.operator.as_str() {
            "Tm" => {
                // Text matrix: a b c d e f - e is X position
                if op.operands.len() >= 6 {
                    current_tm_x = get_operand(&op.operands[4]);
                    current_td_x = 0.0;
                    x_positions.push(current_tm_x);
                }
            }
            "Td" | "TD" => {
                // Text displacement: tx ty
                if op.operands.len() >= 2 {
                    let tx = get_operand(&op.operands[0]);
                    current_td_x += tx;
                    x_positions.push(current_tm_x + current_td_x);
                }
            }
            "T*" => {
                // New line - reset td_x
                current_td_x = 0.0;
            }
            _ => {}
        }
    }

    // Histogram of X positions
    println!("X Coordinate Histogram (25pt bins):\n");
    let mut bins = vec![0usize; 25]; // 0-25, 25-50, ..., 600-625
    for &x in &x_positions {
        let bin = (x / 25.0) as usize;
        if bin < bins.len() {
            bins[bin] += 1;
        }
    }

    for (i, &count) in bins.iter().enumerate() {
        let start = i * 25;
        let end = (i + 1) * 25;
        let bar: String = "*".repeat(count.min(60));
        if count > 0 {
            println!("{:>3}-{:<3}: {:>3} {}", start, end, count, bar);
        }
    }

    // Column analysis
    println!("\n\nColumn Analysis (based on text positions):");
    let center = page_width / 2.0;
    let gutter_start = 280.0;
    let gutter_end = 320.0;

    let left_count = x_positions.iter().filter(|&&x| x < gutter_start).count();
    let gutter_count = x_positions
        .iter()
        .filter(|&&x| x >= gutter_start && x <= gutter_end)
        .count();
    let right_count = x_positions.iter().filter(|&&x| x > gutter_end).count();

    println!(
        "Left column (x < {}): {} positions",
        gutter_start, left_count
    );
    println!(
        "Gutter ({}-{}): {} positions",
        gutter_start, gutter_end, gutter_count
    );
    println!(
        "Right column (x > {}): {} positions",
        gutter_end, right_count
    );
    println!(
        "\nBalance ratio: {:.2}",
        right_count as f32 / (left_count as f32 + 0.01)
    );

    Ok(())
}

fn get_page_content(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let page_dict = doc.get_dictionary(page_id)?;
    let contents = page_dict.get(b"Contents")?;

    match contents {
        lopdf::Object::Stream(stream) => Ok(stream.decompressed_content()?),
        lopdf::Object::Reference(id) => {
            let stream = doc.get_object(*id)?;
            if let lopdf::Object::Stream(s) = stream {
                Ok(s.decompressed_content()?)
            } else {
                Err("Contents not a stream".into())
            }
        }
        lopdf::Object::Array(arr) => {
            let mut combined = Vec::new();
            for obj in arr {
                if let lopdf::Object::Reference(id) = obj {
                    if let Ok(lopdf::Object::Stream(s)) = doc.get_object(*id) {
                        combined.extend(s.decompressed_content()?);
                    }
                }
            }
            Ok(combined)
        }
        _ => Err("Unexpected Contents type".into()),
    }
}

fn get_number(obj: &lopdf::Object) -> Option<f32> {
    match obj {
        lopdf::Object::Integer(n) => Some(*n as f32),
        lopdf::Object::Real(n) => Some(*n),
        _ => None,
    }
}

fn get_operand(obj: &lopdf::Object) -> f32 {
    match obj {
        lopdf::Object::Integer(n) => *n as f32,
        lopdf::Object::Real(n) => *n,
        _ => 0.0,
    }
}
