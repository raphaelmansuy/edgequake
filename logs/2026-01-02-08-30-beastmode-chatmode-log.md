# Task logs

- Actions: Patched PostProcessor to clean spans + normalize span-boundary spaces; added unit tests; ran `cargo test -p edgequake-pdf`; re-ran `real_dataset_eval` multiple times.
- Decisions: Treat span-cleanup as priority because MarkdownRenderer prefers spans; keep broad camelCase split for missing-space joins but add targeted `arXiv` repair.
- Next steps: Add a metric that ignores tables/markdown syntax for `double_space`; implement intra-block hyphenation join for `-\n` patterns; optionally generate `*.mdf.gen` with `--write` for diff review.
- Lessons/insights: Trimming span edges breaks word separation; span-level cleanup must preserve leading/trailing spaces.
