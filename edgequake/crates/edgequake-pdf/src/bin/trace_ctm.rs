//! Trace CTM transforms for text at Y=-6.27

use lopdf::content::Content;
use lopdf::{Document, Object};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path>", args[0]);
        std::process::exit(1);
    }

    let pdf_path = Path::new(&args[1]);
    let doc = Document::load(pdf_path).expect("Failed to load PDF");

    let page_ids = doc.get_pages();
    if let Some((_, &page_id)) = page_ids.iter().next() {
        if let Ok(content_bytes) = doc.get_page_content(page_id) {
            let content = Content::decode(&content_bytes).expect("Failed to decode content");

            // Graphics state
            let mut ctm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut text_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut line_matrix = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut graphics_stack: Vec<[f32; 6]> = Vec::new();

            let mut elements_at_target = Vec::new();

            for op in &content.operations {
                match op.operator.as_str() {
                    "q" => graphics_stack.push(ctm),
                    "Q" => {
                        if let Some(saved) = graphics_stack.pop() {
                            ctm = saved;
                        }
                    }
                    "cm" => {
                        if op.operands.len() >= 6 {
                            let mut m = [0.0f32; 6];
                            for (i, operand) in op.operands.iter().enumerate().take(6) {
                                m[i] = get_number(operand).unwrap_or(0.0);
                            }
                            // Multiply ctm by m
                            ctm = multiply_matrix(ctm, m);
                        }
                    }
                    "BT" => {
                        text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    }
                    "Tm" => {
                        if op.operands.len() >= 6 {
                            for (i, operand) in op.operands.iter().enumerate().take(6) {
                                text_matrix[i] = get_number(operand).unwrap_or(0.0);
                            }
                            line_matrix = text_matrix;
                        }
                    }
                    "Td" | "TD" => {
                        if op.operands.len() >= 2 {
                            let tx = get_number(&op.operands[0]).unwrap_or(0.0);
                            let ty = get_number(&op.operands[1]).unwrap_or(0.0);
                            line_matrix[4] += tx;
                            line_matrix[5] += ty;
                            text_matrix = line_matrix;
                        }
                    }
                    "Tj" => {
                        if !op.operands.is_empty() {
                            if let Object::String(_, _) = &op.operands[0] {
                                // Calculate visual position by applying CTM
                                let raw_x = text_matrix[4];
                                let raw_y = text_matrix[5];
                                let visual_x = ctm[0] * raw_x + ctm[2] * raw_y + ctm[4];
                                let visual_y = ctm[1] * raw_x + ctm[3] * raw_y + ctm[5];

                                // Target Y=-6.27 in raw coords
                                if (raw_y - (-6.27)).abs() < 0.5 {
                                    elements_at_target
                                        .push((raw_x, raw_y, visual_x, visual_y, ctm));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            println!(
                "Elements at raw Y≈-6.27: {} total",
                elements_at_target.len()
            );
            println!("\nCTM values encountered:");
            let mut seen_ctms: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (i, (raw_x, raw_y, vis_x, vis_y, ctm)) in elements_at_target.iter().enumerate() {
                let ctm_str = format!(
                    "[{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}]",
                    ctm[0], ctm[1], ctm[2], ctm[3], ctm[4], ctm[5]
                );
                if !seen_ctms.contains(&ctm_str) {
                    println!("\nCTM: {}", ctm_str);
                    seen_ctms.insert(ctm_str.clone());
                }
                println!(
                    "  [{:2}] raw=({:7.2},{:7.2}) visual=({:7.2},{:7.2})",
                    i + 1,
                    raw_x,
                    raw_y,
                    vis_x,
                    vis_y
                );
            }
        }
    }
}

fn multiply_matrix(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn get_number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(f) => Some(*f),
        _ => None,
    }
}
