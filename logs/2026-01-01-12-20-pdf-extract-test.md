# Test Log: Run extractor on one_tool_2512.20957v2.pdf

Date: 2026-01-01 12:20

Actions:

- Ran `cargo run --example sota_test` to extract `/test-data/real_dataset/one_tool_2512.20957v2.pdf` with current configuration.

Findings:

- The crate currently uses `MockBackend` (external backends removed), so extraction produced an empty output file (0 bytes):
  - Wrote 0 bytes to `/crates/edgequake-pdf/test-data/real_dataset/one_tool_2512.20957v2.md`.
- Compiler error encountered initially (duplicate `Arc` import); fixed and re-ran.

Decisions & Next steps:

- Option A: Re-enable `sota_backend` (lopdf) temporarily to run a real extraction of the PDF and validate the earlier hyphen/merge fixes.
- Option B: Add a lightweight, minimal extraction backend for quick validation if you prefer not to reintroduce lopdf/pdfium.
- Ask: Which option do you prefer? I can re-enable SOTA for a single test run and then remove it again if you'd like.

Notes:

- This test confirms the pipeline runs and writes output, but real extraction requires a real PDF backend (we intentionally removed lopdf/pdfium as requested).
