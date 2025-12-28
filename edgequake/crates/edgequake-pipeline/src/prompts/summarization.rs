//! Summarization prompts for entity and relationship descriptions.
//!
//! Provides SOTA prompts for MapReduce-style summarization of
//! merged entity descriptions.

/// Summarization prompt templates.
#[derive(Debug, Clone, Default)]
pub struct SummarizationPrompts;

impl SummarizationPrompts {
    /// Create a new summarization prompts instance.
    pub fn new() -> Self {
        Self
    }

    /// Build the entity description summarization prompt.
    ///
    /// Used in MapReduce summarization when merging multiple descriptions.
    pub fn entity_summary_prompt<S: AsRef<str>>(
        &self,
        entity_name: &str,
        descriptions: &[S],
    ) -> String {
        let descriptions_text = descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| format!("{}. {}", i + 1, d.as_ref()))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are a helpful assistant responsible for generating a comprehensive summary of the data provided below.
Given one or two entities, and a list of descriptions, all related to the same entity or group of entities.
Please concatenate all of these into a single, comprehensive description. Make sure to include information collected from all the descriptions.
If the provided descriptions are contradictory, please resolve the contradictions and provide a single, coherent summary based on the more complete information.
Make sure the summary is written in third person and is neutral in tone.
The output should be a single paragraph, no longer than 300 words.

#######
---Entities---
{entity_name}

---Description List---
{descriptions_text}
#######

Output:
"#,
            entity_name = entity_name,
            descriptions_text = descriptions_text
        )
    }

    /// Build the relationship description summarization prompt.
    pub fn relationship_summary_prompt<S: AsRef<str>>(
        &self,
        source: &str,
        target: &str,
        descriptions: &[S],
    ) -> String {
        let descriptions_text = descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| format!("{}. {}", i + 1, d.as_ref()))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are a helpful assistant responsible for generating a comprehensive summary of the relationship between two entities.
Given the source and target entities, and a list of descriptions about their relationship.
Please concatenate all descriptions into a single, comprehensive description that captures the nature of the relationship.
If the provided descriptions are contradictory, please resolve the contradictions and provide a single, coherent summary.
Make sure the summary is written in third person and is neutral in tone.
The output should be a single paragraph, no longer than 200 words.

#######
---Source Entity---
{source}

---Target Entity---
{target}

---Relationship Description List---
{descriptions_text}
#######

Output:
"#,
            source = source,
            target = target,
            descriptions_text = descriptions_text
        )
    }

    /// Build a simple summarization prompt for a single text.
    pub fn simple_summary_prompt(&self, text: &str) -> String {
        format!(
            r#"Summarize the following text concisely while keeping all important facts and details.
The output should be a single paragraph, no longer than 300 words.

---Text---
{text}

---Summary---
"#,
            text = text
        )
    }

    /// Build the chunk-level summarization prompt (for MapReduce map phase).
    pub fn chunk_summary_prompt<S: AsRef<str>>(&self, descriptions: &[S]) -> String {
        let descriptions_text = descriptions
            .iter()
            .enumerate()
            .map(|(i, d)| format!("{}. {}", i + 1, d.as_ref()))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"Summarize the following descriptions into a single, comprehensive summary.
Keep all important facts and details. Do not add any information not present in the input.
The output should be a single paragraph.

---Descriptions---
{descriptions_text}

---Summary---
"#,
            descriptions_text = descriptions_text
        )
    }

    /// Build the reduce-level summarization prompt (for combining chunk summaries).
    pub fn reduce_summary_prompt<S: AsRef<str>>(&self, summaries: &[S]) -> String {
        let summaries_text = summaries
            .iter()
            .enumerate()
            .map(|(i, s)| format!("Summary {}: {}", i + 1, s.as_ref()))
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"Combine the following summaries into a single, unified summary.
Merge overlapping information and ensure no important details are lost.
The output should be a single, coherent paragraph.

---Summaries to Combine---
{summaries_text}

---Combined Summary---
"#,
            summaries_text = summaries_text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_summary_prompt() {
        let prompts = SummarizationPrompts::new();
        let descriptions = vec![
            "John is a developer.".to_string(),
            "John works at Acme Corp.".to_string(),
        ];

        let prompt = prompts.entity_summary_prompt("John Doe", &descriptions);

        assert!(prompt.contains("John Doe"));
        assert!(prompt.contains("1. John is a developer."));
        assert!(prompt.contains("2. John works at Acme Corp."));
        assert!(prompt.contains("comprehensive summary"));
    }

    #[test]
    fn test_relationship_summary_prompt() {
        let prompts = SummarizationPrompts::new();
        let descriptions = vec![
            "They collaborate on projects.".to_string(),
            "They have worked together for 5 years.".to_string(),
        ];

        let prompt = prompts.relationship_summary_prompt("Alice", "Bob", &descriptions);

        assert!(prompt.contains("Alice"));
        assert!(prompt.contains("Bob"));
        assert!(prompt.contains("collaborate on projects"));
    }

    #[test]
    fn test_chunk_summary_prompt() {
        let prompts = SummarizationPrompts::new();
        let descriptions = vec!["Fact 1".to_string(), "Fact 2".to_string()];

        let prompt = prompts.chunk_summary_prompt(&descriptions);

        assert!(prompt.contains("1. Fact 1"));
        assert!(prompt.contains("2. Fact 2"));
    }

    #[test]
    fn test_reduce_summary_prompt() {
        let prompts = SummarizationPrompts::new();
        let summaries = vec![
            "Summary A content.".to_string(),
            "Summary B content.".to_string(),
        ];

        let prompt = prompts.reduce_summary_prompt(&summaries);

        assert!(prompt.contains("Summary 1: Summary A content"));
        assert!(prompt.contains("Summary 2: Summary B content"));
        assert!(prompt.contains("Combine"));
    }
}
