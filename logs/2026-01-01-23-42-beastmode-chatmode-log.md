# Task logs

- Actions: Fixed `TextTableReconstructionProcessor` placement/compilation; wired it into the processor chain; regenerated `2900_Goyal_et_al.mdf`; improved row parsing; ran `cargo test -p edgequake-pdf`.
- Decisions: Prefer caption-anchored text reconstruction over re-enabling layout table detection (kept disabled) to avoid malformed output regressions.
- Next steps: If needed, tighten heuristics for mixed/merged rows (e.g., A2.3) and add a focused regression test around `real_dataset/2900_Goyal_et_al.pdf` Table 1.
- Lessons/insights: The renderer already supports tables; the missing piece was producing `BlockType::Table` with populated `TableCell` children early enough in the pipeline.
