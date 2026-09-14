//! Content-stream signal walk for modality classification (SPEC-134).
//!
//! First principles: "is this page a scan?" is answered by *where images are
//! placed* and *how much real glyph text exists* — not by decoding pixels. A
//! full-page scan paints one image (or overlapping tiles) over the whole
//! page; its text layer (when present) is an invisible OCR byproduct. A
//! born-digital page places figures in bounded regions and carries thousands
//! of real glyphs.
//!
//! One pass over the content stream extracts both signals:
//!
//! ```text
//!  ops ── q/Q/cm/Do/BI ──► CTM stack ──► image placement bboxes (pt²)
//!      └─ Tj/TJ/'/" ─────► string bytes ─► text char count
//! ```
//!
//! Cost is O(operators): no font analysis, no image decode, no page raster.
//! Measured ~15–21 ms/page on pathological inputs vs ~1.6–2.8 s/page for the
//! full semantic chunk parser (~130× faster) and 41.5 s for a decode-based
//! image pass.
//!
//! DRY: CTM math reuses [`edgeparse_core::pdf::graphics_state::Matrix`]; the
//! unit-square → bbox transform mirrors `chunk_parser::emit_image_from_ctm`.

use edgeparse_core::pdf::graphics_state::Matrix;
use lopdf::content::Content;
use lopdf::{Dictionary, Document, Object, ObjectId};

/// Per-page signals collected in a single content-stream walk.
#[derive(Debug, Clone, PartialEq)]
pub struct PageWalkSignals {
    /// 1-indexed page number.
    pub page_num: usize,
    /// Image placement bboxes `(x0, y0, x1, y1)` in PDF points.
    pub image_bboxes: Vec<(f64, f64, f64, f64)>,
    /// Native `/Width`×`/Height` for every Image `Do` (including degenerate
    /// placements). Used to skip pdfium decode of HTML table-border storms.
    pub image_native: Vec<(u32, u32)>,
    /// Approximate glyph count from text-showing operators (string bytes;
    /// multi-byte encodings overcount — acceptable for a density heuristic).
    pub text_chars: usize,
}

/// Max Form XObject nesting depth (mirrors chunk_parser's recursion bound).
const MAX_FORM_DEPTH: u8 = 4;

/// Walk the given 1-indexed pages, one signals struct per page.
///
/// Unparseable pages contribute an empty signals struct (fail-open: the
/// classifier then sees a blank page → Print-leaning, the safe default).
pub fn walk_page_signals(doc: &Document, page_nums: &[usize]) -> Vec<PageWalkSignals> {
    walk_page_signals_inner(doc, page_nums, false)
}

/// Like [`walk_page_signals`], but stop a page once its Image `Do` list is a
/// decode storm (HTML table-border pages have thousands of 5×1 XObjects).
/// Used by extract gating — modality classification must not abort.
pub fn walk_page_signals_abort_storm(doc: &Document, page_nums: &[usize]) -> Vec<PageWalkSignals> {
    walk_page_signals_inner(doc, page_nums, true)
}

fn walk_page_signals_inner(
    doc: &Document,
    page_nums: &[usize],
    abort_on_storm: bool,
) -> Vec<PageWalkSignals> {
    let pages = doc.get_pages();
    page_nums
        .iter()
        .map(|&pn| {
            let mut signals = PageWalkSignals {
                page_num: pn,
                image_bboxes: Vec::new(),
                image_native: Vec::new(),
                text_chars: 0,
            };
            if let Some(&page_id) = pages.get(&(pn as u32)) {
                walk_page(doc, &mut signals, page_id, abort_on_storm);
            }
            signals
        })
        .collect()
}

fn walk_page(
    doc: &Document,
    signals: &mut PageWalkSignals,
    page_id: ObjectId,
    abort_on_storm: bool,
) {
    // Page resources (handles /Resources inheritance from parent Pages node).
    // The dictionary comes back inline when direct, or as object ids when
    // indirect — resolve the first id that yields a dictionary.
    let page_resources = match doc.get_page_resources(page_id) {
        Ok((dict, ids)) => dict.cloned().or_else(|| {
            ids.iter().find_map(|&id| {
                doc.get_object(id)
                    .ok()
                    .and_then(|o| o.as_dict().ok().cloned())
            })
        }),
        Err(_) => None,
    };

    // Decode each content stream separately: `Document::get_page_content`
    // concatenates stream bytes without a separator, which fuses boundary
    // tokens (e.g. `EMC` + `q` → `EMCq`) and silently corrupts q/Q balance.
    // The graphics-state stack persists across streams (PDF 32000-1 §7.8.1),
    // so one stack is shared while tokenization stays per-stream.
    let mut ctm_stack = vec![Matrix::identity()];
    for stream_id in doc.get_page_contents(page_id) {
        let Ok(obj) = doc.get_object(stream_id) else {
            continue;
        };
        let Ok(stream) = obj.as_stream() else {
            continue;
        };
        let data = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());
        let Ok(content) = Content::decode(&data) else {
            continue;
        };
        if walk_ops(
            doc,
            &content.operations,
            page_resources.as_ref(),
            &mut ctm_stack,
            signals,
            0,
            abort_on_storm,
        ) {
            break;
        }
    }
}

fn walk_ops(
    doc: &Document,
    ops: &[lopdf::content::Operation],
    resources: Option<&Dictionary>,
    ctm_stack: &mut Vec<Matrix>,
    signals: &mut PageWalkSignals,
    depth: u8,
    abort_on_storm: bool,
) -> bool {
    for op in ops {
        match op.operator.as_str() {
            "q" => {
                let top = ctm_stack.last().copied().unwrap_or_else(Matrix::identity);
                ctm_stack.push(top);
            }
            "Q" if ctm_stack.len() > 1 => {
                ctm_stack.pop();
            }
            "cm" => {
                if let Some(m) = matrix_from_operands(&op.operands) {
                    let top = ctm_stack.last().copied().unwrap_or_else(Matrix::identity);
                    // PDF 32000-1 §8.3.4: cm concatenates as CTM' = M × CTM.
                    let next = m.multiply(&top);
                    if let Some(slot) = ctm_stack.last_mut() {
                        *slot = next;
                    }
                }
            }
            "Do" => {
                let Some(name) = op.operands.first().and_then(as_name) else {
                    continue;
                };
                let Some(xobj) = lookup_xobject(doc, resources, name) else {
                    continue;
                };
                let Ok(stream) = xobj.as_stream() else {
                    continue;
                };
                match subtype_of(&stream.dict) {
                    Some("Image") => {
                        let native = image_native_size(&stream.dict);
                        emit_placement(ctm_stack.last(), signals, Some(native));
                        if abort_on_storm
                            && crate::figure_keep::is_decode_storm(&signals.image_native)
                        {
                            return true;
                        }
                    }
                    Some("Form")
                        if depth < MAX_FORM_DEPTH
                            && walk_form(
                                doc,
                                stream,
                                resources,
                                ctm_stack,
                                signals,
                                depth,
                                abort_on_storm,
                            ) =>
                    {
                        return true;
                    }
                    _ => {}
                }
            }
            // Inline image begins at the current CTM. Native size is unknown.
            "BI" => emit_placement(ctm_stack.last(), signals, None),
            // Text-showing operators: Tj (string), TJ (array), ' and "
            // (move-and-show, string is the last operand).
            "Tj" | "'" | "\"" => {
                if let Some(s) = op.operands.first().and_then(as_string_len) {
                    signals.text_chars += s;
                }
            }
            "TJ" => {
                if let Some(Object::Array(items)) = op.operands.first() {
                    signals.text_chars += items.iter().filter_map(as_string_len).sum::<usize>();
                }
            }
            _ => {}
        }
    }
    false
}

fn walk_form(
    doc: &Document,
    stream: &lopdf::Stream,
    page_resources: Option<&Dictionary>,
    ctm_stack: &mut Vec<Matrix>,
    signals: &mut PageWalkSignals,
    depth: u8,
    abort_on_storm: bool,
) -> bool {
    // Form /Matrix concatenates onto the current CTM (default identity).
    let form_matrix = stream
        .dict
        .get(b"Matrix")
        .ok()
        .and_then(|o| as_f64_vec(doc, o))
        .and_then(|v| matrix_from_slice(&v))
        .unwrap_or_else(Matrix::identity);
    let parent = ctm_stack.last().copied().unwrap_or_else(Matrix::identity);
    // Same concatenation order as cm: CTM' = M_form × CTM.
    ctm_stack.push(form_matrix.multiply(&parent));

    // Form may carry its own /Resources; fall back to the page's.
    let form_resources = stream
        .dict
        .get(b"Resources")
        .ok()
        .and_then(|o| deref(doc, o))
        .and_then(|o| o.as_dict().ok().cloned());
    let effective = form_resources.as_ref().or(page_resources);

    let mut aborted = false;
    if let Ok(data) = stream.decompressed_content() {
        if let Ok(content) = Content::decode(&data) {
            aborted = walk_ops(
                doc,
                &content.operations,
                effective,
                ctm_stack,
                signals,
                depth + 1,
                abort_on_storm,
            );
        }
    }
    ctm_stack.pop();
    aborted
}

/// Emit the CTM-transformed unit square as an axis-aligned bbox.
///
/// An image occupies [0,0]–[1,1] in its own space before the CTM applies
/// (PDF 32000-1 §8.3.24), so the placement bbox is the transformed square.
fn emit_placement(ctm: Option<&Matrix>, signals: &mut PageWalkSignals, native: Option<(u32, u32)>) {
    if let Some(native) = native {
        signals.image_native.push(native);
    }
    let Some(ctm) = ctm else { return };
    let (x0, y0) = ctm.transform_point(0.0, 0.0);
    let (x1, y1) = ctm.transform_point(1.0, 0.0);
    let (x2, y2) = ctm.transform_point(1.0, 1.0);
    let (x3, y3) = ctm.transform_point(0.0, 1.0);
    let min_x = x0.min(x1).min(x2).min(x3);
    let max_x = x0.max(x1).max(x2).max(x3);
    let min_y = y0.min(y1).min(y2).min(y3);
    let max_y = y0.max(y1).max(y2).max(y3);
    // Skip degenerate (zero-area) placements from the *area* signal.
    // Native sizes are still recorded so table-border storms are visible.
    if (max_x - min_x).abs() < 0.1 || (max_y - min_y).abs() < 0.1 {
        return;
    }
    signals.image_bboxes.push((min_x, min_y, max_x, max_y));
}

fn image_native_size(dict: &Dictionary) -> (u32, u32) {
    let w = dict.get(b"Width").ok().and_then(as_u32).unwrap_or(0);
    let h = dict.get(b"Height").ok().and_then(as_u32).unwrap_or(0);
    (w, h)
}

fn as_u32(obj: &Object) -> Option<u32> {
    match obj {
        Object::Integer(i) if *i >= 0 => u32::try_from(*i).ok(),
        Object::Real(f) if *f >= 0.0 => Some(*f as u32),
        _ => None,
    }
}

fn lookup_xobject<'a>(
    doc: &'a Document,
    resources: Option<&'a Dictionary>,
    name: &[u8],
) -> Option<&'a Object> {
    let xobjects = resources?
        .get(b"XObject")
        .ok()
        .and_then(|o| deref(doc, o))?
        .as_dict()
        .ok()?;
    xobjects.get(name).ok().and_then(|o| deref(doc, o))
}

fn subtype_of(dict: &Dictionary) -> Option<&str> {
    match dict.get(b"Subtype").ok()? {
        Object::Name(n) => Some(std::str::from_utf8(n).unwrap_or("")),
        _ => None,
    }
}

fn matrix_from_operands(operands: &[Object]) -> Option<Matrix> {
    if operands.len() != 6 {
        return None;
    }
    let v: Vec<f64> = operands.iter().filter_map(as_f64).collect();
    matrix_from_slice(&v)
}

fn matrix_from_slice(v: &[f64]) -> Option<Matrix> {
    if v.len() != 6 {
        return None;
    }
    Some(Matrix {
        a: v[0],
        b: v[1],
        c: v[2],
        d: v[3],
        e: v[4],
        f: v[5],
    })
}

fn as_name(obj: &Object) -> Option<&[u8]> {
    match obj {
        Object::Name(n) => Some(n),
        _ => None,
    }
}

fn as_f64(obj: &Object) -> Option<f64> {
    match obj {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(f) => Some(*f),
        _ => None,
    }
}

fn as_string_len(obj: &Object) -> Option<usize> {
    match obj {
        Object::String(bytes, _) => Some(bytes.len()),
        _ => None,
    }
}

fn as_f64_vec(doc: &Document, obj: &Object) -> Option<Vec<f64>> {
    let arr = deref(doc, obj)?.as_array().ok()?;
    Some(arr.iter().filter_map(as_f64).collect())
}

/// Follow a reference chain via the document (lopdf caps deref loops).
fn deref<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    doc.dereference(obj).ok().map(|(_, o)| o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_multiply_translate() {
        let t = Matrix::translate(10.0, 20.0);
        let (x, y) = t.transform_point(1.0, 1.0);
        assert_eq!((x, y), (11.0, 21.0));
    }

    #[test]
    fn matrix_from_operands_requires_six() {
        assert!(matrix_from_slice(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]).is_some());
        assert!(matrix_from_slice(&[1.0, 0.0]).is_none());
    }

    #[test]
    fn string_len_only_for_strings() {
        assert_eq!(as_string_len(&Object::Integer(42)), None);
        assert_eq!(
            as_string_len(&Object::String(
                b"abc".to_vec(),
                lopdf::StringFormat::Literal
            )),
            Some(3)
        );
    }
}
