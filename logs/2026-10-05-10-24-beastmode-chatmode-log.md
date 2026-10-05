Actions: Measured unforced entity and ANN plans; fixed scope lookups, typed ANN score ordering and deadlines; validated selected regressions; saved query/index evidence.
Decisions: Reuse migrated indexes and shared query/error/session helpers; preserve NULL scope semantics; record Parse/Describe and runtime coverage limits explicitly.
Next steps: Verify the commit; measure production dimensions, data distribution and sustained concurrency before claiming global index optimality.
Lessons/insights: Scope-specialized equality reduced entity buffer work by 98.84%; halfvec index ordering required a stored-score reorder; passing legacy contract counts include two reported soft skips.
