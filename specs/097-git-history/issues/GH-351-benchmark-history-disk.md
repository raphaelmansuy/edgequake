# Issue study — GH-351 Benchmark history disk storage

> **Issue**: https://github.com/raphaelmansuy/edgequake/issues/351  
> **Reporter**: [@msc2106](https://github.com/msc2106) (2026-07-29)  
> **Owner**: will fix for next release ([comment](https://github.com/raphaelmansuy/edgequake/issues/351#issuecomment-5144508196))  
> **SPEC**: [SPEC-097](../README.md)

## Reporter ask

> Is it necessary to include benchmark run results in version control?

## Answer (first principles)

| Artifact class | In VCS? | Why |
|----------------|---------|-----|
| Thin scorecard / SUMMARY / BUSINESS_REPORT / publish peers | **Yes** | Small Acc SSOT; doc links |
| Fat predictions / eval / raw / progress.jsonl | **No** | Regenerable; dominate clone size |
| Build ghosts (`sdks/swift/.build`, `zz_test_docs`) | **No** | Already tip-ignored; strip from history |

## Verification of claims

Reporter’s path and scale match local measurement: ~4.4 GB under `history/`, per-run mid ~100 MB+, smoke tens of MB, 160+ folders. GitHub repo size ~717 MB (compressed pack); tip checkout still materializes multi-GB trees when fat is present.

## Acceptance for close

1. Fat globs untracked (G1).  
2. History rewritten; history-path blob sum thin-only (G3).  
3. Guard + CONTRIBUTING prevent recurrence (G4/G5).  
4. Comment on #351 with before/after `size-pack` / GitHub size.
