//! Recursive character chunking (LightRAG strategy `R` parity — SPEC-026 Phase 2).
//!
//! Mirrors LangChain `RecursiveCharacterTextSplitter` control flow and
//! `lightrag/chunker/recursive_character.py` span-preserving merge.

use async_trait::async_trait;

use super::text_utils::{estimate_tokens, floor_char_boundary};
use super::types::{ChunkResult, ChunkerConfig, ChunkingStrategy};
use crate::error::Result;

type SpanPiece = (String, usize, usize);

/// LightRAG production default separator cascade (Jun 2026).
pub fn default_recursive_separators() -> Vec<String> {
    vec![
        "\n\n".to_string(),
        "\n".to_string(),
        "。".to_string(),
        "！".to_string(),
        "？".to_string(),
        "；".to_string(),
        "，".to_string(),
        " ".to_string(),
        String::new(),
    ]
}

/// Token length for recursive merge decisions — aligned with LightRAG's
/// tokenizer-backed `length_function` (word-aware Latin, char-aware CJK).
fn recursive_token_len(text: &str) -> usize {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return 0;
    }

    let cjk_chars = trimmed.chars().filter(|c| is_cjk_char(*c)).count();
    let mut base = if cjk_chars * 2 >= trimmed.chars().count() {
        ((trimmed.chars().count() as f32 / 1.5).ceil() as usize).max(1)
    } else {
        let words = trimmed.split_whitespace().filter(|w| !w.is_empty()).count();
        if words > 0 {
            words
        } else {
            estimate_tokens(trimmed)
        }
    };

    // keep_separator=true: leading paragraph/line breaks count toward merge budget
    // (tiktoken typically counts `\n\n` as ~2 tokens).
    if text.starts_with("\n\n") {
        base += 2;
    } else if text.starts_with('\n') {
        base += 1;
    }

    base.max(1)
}

fn is_cjk_char(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3000..=0x303F)
}

/// Recursive character chunking with token-aware merge and overlap.
pub struct RecursiveCharacterChunking;

impl RecursiveCharacterChunking {
    fn effective_separators(config: &ChunkerConfig) -> Vec<String> {
        if config.separators.is_empty() {
            default_recursive_separators()
        } else {
            config.separators.clone()
        }
    }

    /// Split on literal `separator`, keeping the separator at the start of each
    /// piece after the first (LangChain `keep_separator=true`).
    fn split_literal_with_spans(text: &str, separator: &str, base_offset: usize) -> Vec<SpanPiece> {
        if separator.is_empty() {
            return text
                .char_indices()
                .filter(|(_, ch)| !ch.is_whitespace())
                .map(|(idx, ch)| {
                    let start = base_offset + idx;
                    (ch.to_string(), start, start + ch.len_utf8())
                })
                .collect();
        }

        if !text.contains(separator) {
            if text.is_empty() {
                return Vec::new();
            }
            return vec![(text.to_string(), base_offset, base_offset + text.len())];
        }

        let mut positions = Vec::new();
        let mut cursor = 0usize;
        while cursor < text.len() {
            if let Some(rel) = text[cursor..].find(separator) {
                let sep_start = cursor + rel;
                positions.push(sep_start);
                cursor = sep_start + separator.len();
            } else {
                break;
            }
        }

        let mut pieces = Vec::new();
        if positions[0] > 0 {
            let end = floor_char_boundary(text, positions[0]);
            let piece = text[..end].to_string();
            if !piece.trim().is_empty() {
                pieces.push((piece, base_offset, base_offset + end));
            }
        }

        for (index, &sep_start) in positions.iter().enumerate() {
            let end = positions.get(index + 1).copied().unwrap_or(text.len());
            if end > sep_start {
                let start = floor_char_boundary(text, sep_start);
                let end = floor_char_boundary(text, end);
                let piece = text[start..end].to_string();
                if !piece.trim().is_empty() {
                    pieces.push((piece, base_offset + start, base_offset + end));
                }
            }
        }

        pieces
    }

    fn join_span_pieces(
        pieces: &[SpanPiece],
        separator: &str,
        strip_whitespace: bool,
    ) -> Option<SpanPiece> {
        if pieces.is_empty() {
            return None;
        }

        let mut chars = String::new();
        let mut char_offsets: Vec<usize> = Vec::new();
        for (index, (fragment, start, end)) in pieces.iter().enumerate() {
            if index > 0 && !separator.is_empty() {
                let previous_end = pieces[index - 1].2;
                for (sep_index, sep_char) in separator.chars().enumerate() {
                    chars.push(sep_char);
                    char_offsets.push(previous_end + sep_index);
                }
            }
            chars.push_str(fragment);
            char_offsets.extend(*start..*end);
        }

        let (left, right) = if strip_whitespace {
            let mut l = 0;
            let mut r = chars.len();
            while l < r && chars.as_bytes()[l].is_ascii_whitespace() {
                l += 1;
            }
            while r > l && chars.as_bytes()[r - 1].is_ascii_whitespace() {
                r -= 1;
            }
            (l, r)
        } else {
            (0, chars.len())
        };

        if left >= right {
            return None;
        }

        let text: String = chars.chars().skip(left).take(right - left).collect();
        let start = char_offsets[left];
        let end = char_offsets[right - 1] + text.chars().last().map(|c| c.len_utf8()).unwrap_or(1);
        Some((text, start, end))
    }

    /// Mirror LightRAG `_merge_splits_with_spans`.
    fn merge_splits_with_spans(
        splits: &[SpanPiece],
        separator: &str,
        chunk_size: usize,
        chunk_overlap: usize,
        strip_whitespace: bool,
    ) -> Vec<SpanPiece> {
        if splits.is_empty() {
            return Vec::new();
        }

        let separator_len = if separator.is_empty() {
            0
        } else {
            recursive_token_len(separator)
        };

        let mut docs: Vec<SpanPiece> = Vec::new();
        let mut current_doc: Vec<SpanPiece> = Vec::new();
        let mut total = 0usize;

        for split in splits {
            let split_len = recursive_token_len(&split.0);
            let with_sep = total
                + split_len
                + if current_doc.is_empty() {
                    0
                } else {
                    separator_len
                };

            if with_sep > chunk_size {
                if total > chunk_size {
                    tracing::warn!(
                        total,
                        chunk_size,
                        "recursive chunk exceeded token budget before flush"
                    );
                }
                if !current_doc.is_empty() {
                    if let Some(doc) =
                        Self::join_span_pieces(&current_doc, separator, strip_whitespace)
                    {
                        docs.push(doc);
                    }
                    while total > chunk_overlap
                        || (total
                            + split_len
                            + if current_doc.is_empty() {
                                0
                            } else {
                                separator_len
                            }
                            > chunk_size
                            && total > 0)
                    {
                        let removed_len = recursive_token_len(&current_doc[0].0)
                            + if current_doc.len() > 1 {
                                separator_len
                            } else {
                                0
                            };
                        total = total.saturating_sub(removed_len);
                        current_doc.remove(0);
                    }
                }
            }

            current_doc.push(split.clone());
            total += split_len
                + if current_doc.len() > 1 {
                    separator_len
                } else {
                    0
                };
        }

        if let Some(doc) = Self::join_span_pieces(&current_doc, separator, strip_whitespace) {
            docs.push(doc);
        }
        docs
    }

    /// Mirror LightRAG `_split_text_with_spans`.
    fn split_text_with_spans(
        text: &str,
        base_offset: usize,
        separators: &[String],
        chunk_size: usize,
        chunk_overlap: usize,
    ) -> Vec<SpanPiece> {
        let mut separator = separators.last().cloned().unwrap_or_default();
        let mut rest: &[String] = &[];

        for (index, candidate) in separators.iter().enumerate() {
            if candidate.is_empty() {
                separator = String::new();
                break;
            }
            if text.contains(candidate.as_str()) {
                separator = candidate.clone();
                rest = &separators[index + 1..];
                break;
            }
        }

        let splits = if separator.is_empty() {
            Self::split_literal_with_spans(text, "", base_offset)
        } else {
            Self::split_literal_with_spans(text, &separator, base_offset)
        };

        let merge_separator = "";
        let mut final_chunks: Vec<SpanPiece> = Vec::new();
        let mut good_splits: Vec<SpanPiece> = Vec::new();

        for split in splits {
            if recursive_token_len(&split.0) < chunk_size {
                good_splits.push(split);
            } else if !good_splits.is_empty() {
                final_chunks.extend(Self::merge_splits_with_spans(
                    &good_splits,
                    merge_separator,
                    chunk_size,
                    chunk_overlap,
                    true,
                ));
                good_splits.clear();

                if rest.is_empty() {
                    final_chunks.push(split);
                } else {
                    final_chunks.extend(Self::split_text_with_spans(
                        &split.0,
                        split.1,
                        rest,
                        chunk_size,
                        chunk_overlap,
                    ));
                }
            } else if rest.is_empty() {
                final_chunks.push(split);
            } else {
                final_chunks.extend(Self::split_text_with_spans(
                    &split.0,
                    split.1,
                    rest,
                    chunk_size,
                    chunk_overlap,
                ));
            }
        }

        if !good_splits.is_empty() {
            final_chunks.extend(Self::merge_splits_with_spans(
                &good_splits,
                merge_separator,
                chunk_size,
                chunk_overlap,
                true,
            ));
        }

        final_chunks
    }

    fn trim_span_piece(raw: SpanPiece) -> Option<SpanPiece> {
        let (raw_body, mut start_index, mut end_index) = raw;
        let bytes = raw_body.as_bytes();
        let mut left = 0usize;
        let mut right = raw_body.len();
        while left < right && bytes[left].is_ascii_whitespace() {
            left += 1;
        }
        while right > left && bytes[right - 1].is_ascii_whitespace() {
            right -= 1;
        }
        if left >= right {
            return None;
        }
        let body = raw_body[left..right].to_string();
        start_index += left;
        end_index -= raw_body.len() - right;
        Some((body, start_index, end_index))
    }
}

#[async_trait]
impl ChunkingStrategy for RecursiveCharacterChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let separators = Self::effective_separators(config);
        let chunk_size = config.chunk_size.max(1);
        let chunk_overlap = config.chunk_overlap;

        let pieces =
            Self::split_text_with_spans(content, 0, &separators, chunk_size, chunk_overlap);

        Ok(pieces
            .into_iter()
            .filter_map(Self::trim_span_piece)
            .enumerate()
            .map(|(idx, (text, start, end))| ChunkResult {
                content: text.clone(),
                tokens: recursive_token_len(&text),
                chunk_order_index: idx,
                section: None,
                start_offset: Some(start),
                end_offset: Some(end),
                page_start: None,
                page_end: None,
            })
            .filter(|c| !c.content.is_empty())
            .collect())
    }

    fn name(&self) -> &str {
        "recursive_character"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_separators_match_lightrag() {
        assert_eq!(
            default_recursive_separators(),
            vec!["\n\n", "\n", "。", "！", "？", "；", "，", " ", ""]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn splits_on_paragraphs() {
        let text = "Para one.\n\nPara two.\n\nPara three.";
        let config = ChunkerConfig {
            chunk_size: 5,
            chunk_overlap: 0,
            min_chunk_size: 1,
            separators: default_recursive_separators(),
            ..Default::default()
        };
        let chunks = RecursiveCharacterChunking
            .chunk(text, &config)
            .await
            .unwrap();
        assert!(chunks.len() >= 2);
    }

    #[tokio::test]
    async fn plain_en_fixture_matches_lightrag_chunk_count() {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/spec026/plain_en.txt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let config = ChunkerConfig {
            chunk_size: 15,
            chunk_overlap: 0,
            min_chunk_size: 1,
            separators: default_recursive_separators(),
            ..Default::default()
        };
        let chunks = RecursiveCharacterChunking
            .chunk(&text, &config)
            .await
            .unwrap();
        let starts: Vec<_> = chunks.iter().filter_map(|c| c.start_offset).collect();
        assert_eq!(
            chunks.len(),
            3,
            "expected 3 paragraph-aligned chunks at chunk_size=15, got {} starts={starts:?}",
            chunks.len()
        );
    }

    #[test]
    fn recursive_token_len_counts_leading_paragraph_break() {
        let s = "\n\nParagraph two continues the document with more content.";
        assert_eq!(recursive_token_len(s), 10);
    }

    #[test]
    fn merge_plain_en_paragraphs_respects_token_budget() {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/spec026/plain_en.txt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let splits = RecursiveCharacterChunking::split_literal_with_spans(text.as_str(), "\n\n", 0);
        let merged = RecursiveCharacterChunking::merge_splits_with_spans(&splits, "", 15, 0, true);
        assert_eq!(
            merged.len(),
            3,
            "expected 3 merged chunks, got {}: {:?}",
            merged.len(),
            merged.iter().map(|p| (p.1, &p.0)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn split_literal_plain_en_paragraphs() {
        let text = std::fs::read_to_string(format!(
            "{}/tests/fixtures/spec026/plain_en.txt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let pieces = RecursiveCharacterChunking::split_literal_with_spans(text.as_str(), "\n\n", 0);
        assert_eq!(pieces.len(), 3, "expected 3 paragraph splits");
        assert_eq!(pieces[0].1, 0);
        assert_eq!(pieces[1].1, 42);
        assert!(pieces[1].0.starts_with("\n\n"));
        assert_eq!(pieces[2].1, 99);
    }

    #[test]
    fn keep_separator_includes_newlines_on_later_pieces() {
        let text = "Alpha.\n\nBeta.\n\nGamma.";
        let pieces = RecursiveCharacterChunking::split_literal_with_spans(text, "\n\n", 0);
        assert_eq!(pieces.len(), 3);
        assert!(!pieces[0].0.starts_with('\n'));
        assert!(pieces[1].0.starts_with("\n\n"));
    }
}
