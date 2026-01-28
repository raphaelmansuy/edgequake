# Task Log: ASCII Diagram Documentation

**Date**: 2026-01-29  
**Time**: 00:30  
**Mode**: beastmode  
**Task**: Add comprehensive ASCII diagrams to code comments explaining INPUT vs OUTPUT token management

---

## Actions

1. Added 138 lines of comprehensive ASCII diagram documentation to `edgequake/crates/edgequake-pipeline/src/extractor.rs`
2. Replaced brief comments with detailed visual explanations
3. Built and verified code compiles correctly
4. Committed changes with descriptive message (commit: 23870f3a)

## Decisions

- **Location**: Added documentation at lines 698-783 where adaptive max_tokens is calculated
- **Style**: Used box-drawing characters (┌─┐│└─┘) for visual clarity
- **Structure**: Divided into 5 sections:
  1. INPUT side (document chunking) - working correctly
  2. OUTPUT side (LLM response) - the actual problem
  3. KEY INSIGHT - small INPUT ≠ small OUTPUT
  4. SOLUTION - adaptive max_tokens calculation
  5. RETRY STRATEGY - progressive token increase
- **Content**: Used concrete example from user's document (TokenSeek paper)

## Next Steps

- Documentation now clearly explains architectural separation of concerns
- Future developers will understand why INPUT chunking doesn't solve OUTPUT truncation
- Prevents confusion about "we already chunk the document, why do we still have issues?"
- May add similar documentation to orchestrator.rs for INPUT chunking strategy

## Lessons/Insights

**Key architectural insight documented**: INPUT management (chunking) and OUTPUT management (max_tokens) are separate concerns that both require adaptive strategies. A small INPUT chunk (1500 tokens) can generate a large OUTPUT response (9000+ tokens) when content has high entity density (academic papers, technical documentation).

**Why ASCII diagrams**: Visual representation makes complex architectural concepts immediately clear. Developers can see the data flow and understand:

- Document → chunks (INPUT side)
- Chunk → LLM → JSON response (OUTPUT side)
- Retry flow with progressive token increase

**Documentation value**: Answers the natural question "Why do we have this issue? We chunk the document, right?" directly in the code where the fix is implemented. Reduces onboarding time for new developers and prevents future confusion.

---

## Summary

Added 138 lines of comprehensive ASCII diagram documentation explaining the architectural distinction between INPUT token management (document chunking) and OUTPUT token management (LLM response size limits). The documentation uses visual diagrams to show:

1. **INPUT flow**: 137KB document → 5KB chunks → LLM (working correctly)
2. **OUTPUT flow**: Single chunk → 50+ entities + 100+ relationships → 9000 token JSON (the problem)
3. **Key insight**: Small INPUT can generate large OUTPUT (6x multiplier)
4. **Solution**: Adaptive max_tokens (4096-16384) based on chunk complexity
5. **Retry strategy**: Progressive token increase (2x per retry, max 32768)

Commit: `23870f3a` - 1 file changed, 138 insertions, 8 deletions

The documentation directly addresses the user's question "Why do we have this issue? We chunk the document? Right?" by explaining that chunking manages INPUT size, but JSON truncation was an OUTPUT response size issue requiring separate adaptive max_tokens solution.
