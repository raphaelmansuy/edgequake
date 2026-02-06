# OODA Iteration 27 – Decide

## Decision

Fix GarbledTextFilterProcessor to not filter section titles containing `&` and section numbers.

## Changes

1. Add `"&"` to `valid_short_words` array
2. Add section number pattern skip: digit + `)` or `.` → not counted as unusual

## Risk

Low — these are very specific, non-ambiguous patterns. `&` is universally used in English titles. Digit+delimiter is the standard section numbering format.
