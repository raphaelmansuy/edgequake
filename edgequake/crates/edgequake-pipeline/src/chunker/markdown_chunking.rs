//! Markdown-aware chunking with heading breadcrumbs (SPEC-026 Phase 2 P-03).

use async_trait::async_trait;

use super::recursive::RecursiveCharacterChunking;
use super::text_utils::estimate_tokens;
use super::types::{ChunkResult, ChunkerConfig, ChunkingStrategy, SectionMetadata};
use crate::error::Result;
use crate::markdown_ir::{extract_markdown_blocks, format_breadcrumb};

/// Split markdown at heading boundaries; attach section metadata to each chunk.
pub struct MarkdownChunking;

#[async_trait]
impl ChunkingStrategy for MarkdownChunking {
    async fn chunk(&self, content: &str, config: &ChunkerConfig) -> Result<Vec<ChunkResult>> {
        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        let blocks = extract_markdown_blocks(content);
        let recursive = RecursiveCharacterChunking;
        let mut results = Vec::new();
        let mut order = 0usize;

        for block in blocks {
            let section = Some(SectionMetadata::from_block(
                &block.parent_headings,
                &block.heading,
                block.level,
            ));
            let _breadcrumb = format_breadcrumb(&block.parent_headings, &block.heading);

            let sub_chunks = recursive.chunk(&block.content, config).await?;
            if sub_chunks.is_empty() && !block.content.trim().is_empty() {
                results.push(ChunkResult {
                    content: block.content.trim().to_string(),
                    tokens: estimate_tokens(&block.content),
                    chunk_order_index: order,
                    section: section.clone(),
                    ..Default::default()
                });
                order += 1;
            } else {
                for mut sub in sub_chunks {
                    sub.chunk_order_index = order;
                    sub.section = section.clone();
                    order += 1;
                    results.push(sub);
                }
            }
        }

        Ok(results)
    }

    fn name(&self) -> &str {
        "markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn markdown_chunk_carries_section_metadata() {
        let md = "# Guide\n\nIntro text.\n\n## Setup\n\nSetup steps here.";
        let config = ChunkerConfig {
            chunk_size: 200,
            chunk_overlap: 10,
            ..Default::default()
        };
        let chunks = MarkdownChunking.chunk(md, &config).await.unwrap();
        assert!(!chunks.is_empty());
        let setup = chunks
            .iter()
            .find(|c| {
                c.section
                    .as_ref()
                    .is_some_and(|s| s.heading_path.contains(&"Setup".to_string()))
            })
            .expect("setup section chunk");
        assert!(setup
            .section
            .as_ref()
            .unwrap()
            .heading_path
            .contains(&"Guide".to_string()));
    }
}
