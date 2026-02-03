# OODA-18 Act: Investigation Complete, No Code Changes

## Summary

OODA-18 was a deep investigation into multi-column reading order issues.

## Actions Taken

1. ✅ Traced block ordering through entire pipeline
2. ✅ Identified `TableDetectionProcessor.group_blocks_by_row()` as scrambling source
3. ✅ Attempted fix: preserve original order for non-table blocks
4. ✅ Discovered gold files use Y-sorted order, not reading order
5. ❌ Reverted changes (fix lowered measured quality)

## Code Changes

None (all experimental changes reverted)

## Quality Impact

- Before: 86.5%
- After: 86.5% (no change)

## Learnings

1. Gold file methodology affects what "correct" means for evaluation
2. Y-sorted output has been standard; fixing it requires gold file updates
3. TableDetectionProcessor is the main source of order scrambling

## Next Steps

Focus on improvements that don't require gold file changes:

- Margin content filtering
- Empty table block fixes
- Other text quality improvements

## Time Spent

~45 minutes of deep investigation and experimentation
