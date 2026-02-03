# OODA-23 Observation

## Problem Identified

Figure captions like "Fig. 1. Key Components of an Agent's LLM Architecture" were being incorrectly classified as H3 section headers in the markdown output.

## Data Points

- `agent_2510.09244v1.pdf` generated output had 5 figure captions as H3 headers
- Gold file has 31 H3 headings, generated had 56 (before fix)
- Generated markdown showed:
  ```
  ### Fig. 1. Key Components of an Agent's LLM Architecture
  ### Fig. 2. Architecture of Multimodal Large Language Models (MM-LLMs)...
  ### Fig. 3. Usage of segmentation and depth maps for MM-LLM perception [28]
  ### Fig. 4. Image with Set-of-Mark [64]
  ### Fig. 8. Example of the communication between agents in a multi-agent system
  ```

## Root Cause Analysis

The heading classification happens in THREE places in the processing pipeline:

1. `heading_classifier.rs` - font-based classification (Strategy 4 in processor.rs)
2. `processor.rs:StyleDetectionProcessor` - font ratio and pattern-based detection
3. `structure_detection.rs:HeaderDetectionProcessor` - subsection patterns and font-based

Figure captions like "Fig. 1. Key Components..." pass through these filters because:

- They have title-case text (first letter uppercase, contains lowercase)
- They are short (< 80 chars) and don't end with a period (caption ends with title text)
- Font size is often larger than body text (≥1.1x ratio)
- First character is uppercase

The initial fix in `heading_classifier.rs` wasn't being reached because the captions were being classified EARLIER in `structure_detection.rs` and `processor.rs:StyleDetectionProcessor`.
