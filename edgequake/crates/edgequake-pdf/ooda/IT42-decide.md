# OODA-IT42 Decide: Disable Table Detection Pipeline

## Decision
Disable `TableDetectionProcessor` and `TextTableReconstructionProcessor` until proper table reconstruction can be implemented.

## Expected Outcomes
1. Tables render as plain text paragraphs
2. No garbled markdown with wrong data in wrong columns  
3. Content preserved (file size increases as table structure becomes text)
4. IT40/IT41 word spacing fixes remain intact

## Validation Plan
1. Convert LightRAG PDF - verify Tables 1/2/3 are plain text, not garbled
2. Convert Elitizon PDF - verify spacing still correct ("Executive summary" not "Executivesummary")
3. Run clippy - verify 0 warnings
4. Run test suite - verify 462 tests pass
