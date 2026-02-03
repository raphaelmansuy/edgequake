# Task Log: OODA-28 Coordinate System Fix

## Actions

- Fixed MarginFilterProcessor coordinate system (Y=0 at top, not bottom)
- Added running header exemption for large blocks (bbox height > 12pt)
- Added spanning detection heuristic in reading_order.rs for titles
- Committed fix with detailed root cause analysis
- Verified title now appears correctly in one_tool document output

## Decisions

- Used bbox height as proxy for font size (titles typically > 12pt)
- Changed coordinate thresholds: header_threshold = 5% (low Y), footer_threshold = 95% (high Y)
- Kept spanning heuristic in reading_order.rs for additional title detection

## Next Steps

- Quality at 87.6%, target 95% (gap: 7.4pp)
- agent_2510 (81.9%) has complex issues: authors as table, paragraph line breaks
- Need to investigate TextTableReconstructionProcessor false positives
- Consider improving BlockMergeProcessor for paragraph continuation

## Lessons/Insights

- Backend normalizes coordinates to Y=0 at top, but MarginFilterProcessor was using old PDF coordinates
- Running header detection was filtering actual titles because normalized text matches
- Large blocks (title) have bbox height ~14pt vs small blocks (running header) ~9pt
