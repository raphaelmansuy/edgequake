# EdgeQuake PDF Test Data Index

This directory contains a set of PDF documents designed to test the accuracy and effectiveness of the `edgequake-pdf` conversion tool.

## Test Documents

| File                                                       | Purpose                   | Expected Outcome                                                 |
| ---------------------------------------------------------- | ------------------------- | ---------------------------------------------------------------- |
| [001_simple_text.pdf](001_simple_text.pdf)                 | Basic text extraction     | Two distinct paragraphs in Markdown.                             |
| [002_headers_and_lists.pdf](002_headers_and_lists.pdf)     | Header and list detection | Correct ATX headers (#, ##, ###) and Markdown lists (- and 1.).  |
| [003_two_columns.pdf](003_two_columns.pdf)                 | Multi-column layout       | Text read in correct order (Column 1 then Column 2).             |
| [004_tables.pdf](004_tables.pdf)                           | Table extraction          | Properly formatted Markdown table.                               |
| [005_mixed_styles.pdf](005_mixed_styles.pdf)               | Font styles               | Bold, Italic, and Monospace detection.                           |
| [006_images_and_captions.pdf](006_images_and_captions.pdf) | Images and captions       | Graceful handling of images and correct caption extraction.      |
| [007_nested_lists.pdf](007_nested_lists.pdf)               | Nested lists              | Correct indentation and list markers for nested items.           |
| [008_multi_page.pdf](008_multi_page.pdf)                   | Multi-page documents      | Correct flow across pages and potential header/footer filtering. |
| [009_code_blocks.pdf](009_code_blocks.pdf)                 | Code blocks               | Extraction of code snippets with preserved formatting.           |

## How to run tests

Use the CLI tool to convert these files:

```bash
cargo run --bin edgequake-pdf -- convert -i test-data/001_simple_text.pdf
```
