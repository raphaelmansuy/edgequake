//! SPEC-180 08: a hermetic Vertex `:embedContent` route.
//!
//! The vector is a deterministic bag of hashed words, so retrieval tests are red or green
//! without Google:
//!
//! - a text part contributes one hashed vector per word (task prefixes are stripped);
//! - an image part whose bytes carry the marker `concept:<word>;` contributes the vector
//!   of `<word>`, so a text query naming that word lands next to the image;
//! - an image without a marker contributes a hash of its bytes (same bytes, same vector).
//!
//! The result is L2-normalised and has the requested `outputDimensionality`.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

use crate::FakeLlmState;

const DEFAULT_DIMENSION: usize = 3072;
const CONCEPT_MARKER: &[u8] = b"concept:";
/// Words of the document and query prefixes; they carry no meaning.
const PREFIX_WORDS: [&str; 7] = ["title", "none", "text", "task", "search", "result", "query"];

/// FNV-1a, 64 bit.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}

/// A pseudo-random vector in [-1, 1] seeded by `seed` (xorshift64*).
fn seeded_vector(seed: u64, dimension: usize) -> Vec<f32> {
    let mut state = seed | 1;
    (0..dimension)
        .map(|_| {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let bits = state.wrapping_mul(0x2545F4914F6CDD1D) >> 40;
            (bits as f32 / (1u64 << 23) as f32) - 1.0
        })
        .collect()
}

fn word_vector(word: &str, dimension: usize) -> Vec<f32> {
    seeded_vector(fnv(word.as_bytes()), dimension)
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

/// The concept word an image's bytes announce, if any.
fn image_concept(bytes: &[u8]) -> Option<String> {
    let at = bytes
        .windows(CONCEPT_MARKER.len())
        .position(|w| w == CONCEPT_MARKER)?;
    let rest = &bytes[at + CONCEPT_MARKER.len()..];
    let end = rest.iter().position(|b| *b == b';')?;
    let word = String::from_utf8_lossy(&rest[..end]).trim().to_lowercase();
    (!word.is_empty()).then_some(word)
}

fn add_into(total: &mut [f32], vector: &[f32]) {
    total.iter_mut().zip(vector).for_each(|(t, v)| *t += v);
}

fn add_text(total: &mut [f32], text: &str) {
    for word in words(text).filter(|w| !PREFIX_WORDS.contains(&w.as_str())) {
        add_into(total, &word_vector(&word, total.len()));
    }
}

fn add_image(total: &mut [f32], b64: &str) {
    let bytes = B64.decode(b64).unwrap_or_default();
    let vector = match image_concept(&bytes) {
        Some(word) => word_vector(&word, total.len()),
        None => seeded_vector(fnv(&bytes), total.len()),
    };
    add_into(total, &vector);
}

/// The vector of one `content.parts` list.
pub fn embed_parts(parts: &[Value], dimension: usize) -> Vec<f32> {
    let mut total = vec![0.0f32; dimension];
    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            add_text(&mut total, text);
        } else if let Some(data) = part.pointer("/inlineData/data").and_then(Value::as_str) {
            add_image(&mut total, data);
        }
    }
    let norm = total.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        total.iter_mut().for_each(|v| *v /= norm);
    } else {
        total[0] = 1.0;
    }
    total
}

/// `POST /v1/projects/{p}/locations/{l}/publishers/google/models/{model}:embedContent`.
pub async fn embed_content(
    State(state): State<FakeLlmState>,
    Path((_project, _location, model_action)): Path<(String, String, String)>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    state.hits.fetch_add(1, Ordering::Relaxed);
    if !model_action.ends_with(":embedContent") {
        return StatusCode::NOT_FOUND.into_response();
    }
    if !crate::auth_ok(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let dimension = body
        .pointer("/embedContentConfig/outputDimensionality")
        .and_then(Value::as_u64)
        .map(|d| d as usize)
        .filter(|d| *d > 0)
        .unwrap_or(DEFAULT_DIMENSION);
    let parts = body
        .pointer("/content/parts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if parts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": { "message": "content.parts is empty" } })),
        )
            .into_response();
    }
    Json(json!({ "embedding": { "values": embed_parts(&parts, dimension) } })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    fn text(t: &str) -> Value {
        json!({ "text": t })
    }

    fn image(marker: &str) -> Value {
        let mut bytes = b"\x89PNG-fake".to_vec();
        bytes.extend_from_slice(marker.as_bytes());
        json!({ "inlineData": { "mimeType": "image/png", "data": B64.encode(bytes) } })
    }

    #[test]
    fn a_text_query_lands_next_to_the_image_that_names_its_word() {
        let kestrel = embed_parts(&[image("concept:kestrel;")], 768);
        let cat = embed_parts(&[image("concept:cat;")], 768);
        let query = embed_parts(
            &[text("task: search result | query: a photo of a kestrel")],
            768,
        );
        assert!(cosine(&query, &kestrel) > 0.3);
        assert!(cosine(&query, &cat).abs() < 0.15);
    }

    #[test]
    fn the_same_image_bytes_give_the_same_vector() {
        let a = embed_parts(&[image("no marker one")], 768);
        let b = embed_parts(&[image("no marker one")], 768);
        let c = embed_parts(&[image("no marker two")], 768);
        assert!((cosine(&a, &b) - 1.0).abs() < 1e-4);
        assert!(cosine(&a, &c).abs() < 0.2);
    }

    #[test]
    fn vectors_are_unit_length_at_the_requested_width() {
        let v = embed_parts(&[text("title: none | text: hello world")], 1536);
        assert_eq!(v.len(), 1536);
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn prefix_words_carry_no_meaning() {
        let a = embed_parts(&[text("title: none | text: kestrel nest")], 768);
        let b = embed_parts(&[text("task: search result | query: kestrel nest")], 768);
        assert!((cosine(&a, &b) - 1.0).abs() < 1e-4);
    }
}
