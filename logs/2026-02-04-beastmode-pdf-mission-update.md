# Task Log: PDF Mission File Update

**Date:** 2026-02-04
**Mode:** Beastmode
**Task:** Amend mission file for speed/quality focus with Python tool research

## Actions

1. Fetched and analyzed 4 leading Python PDF-to-Markdown tools:
   - **Marker** (31K⭐): Pipeline architecture, LLM hybrid mode, 122 pages/s throughput
   - **Docling** (52K⭐): Heron layout model, VLM support, unified document format
   - **PyMuPDF4LLM** (1.3K⭐): Lightweight, fast, character-level bbox extraction
   - **MarkItDown** (86K⭐): MCP server, streaming API, plugin system

2. Updated mission file with:
   - Primary goals table (Speed P0, Quality P1)
   - Python tools lessons learned section
   - Speed optimization strategy with O(n) algorithm targets
   - Micro-test architecture (Tier 0: <0.1s per test)
   - Benchmark protocol against Python tools
   - Quality mode tiers (Fast/Balanced/Quality/LLM-Enhanced)
   - Updated technical priorities with speed as P0

3. Committed changes: SHA `947035fe` (+422 lines, -43 lines)

## Decisions

- Speed optimization is now **Priority 0 (Critical)** - target <1s per page
- Micro-tests use `include_bytes!` for zero file I/O overhead
- Test pyramid now has 4 tiers instead of 3 (added micro-tests)
- Benchmarking against Python tools is required for validation

## Next Steps

1. Implement micro-tests (`tests/micro_*.rs`) with tiny embedded PDFs
2. Profile current extraction to identify O(n²) hotspots
3. Run benchmark against PyMuPDF4LLM to establish baseline speed
4. Port Marker's lattice table detection for improved SFS

## Lessons/Insights

- Marker achieves 122 pages/s on H100 with batch mode - our target should be competitive
- PyMuPDF4LLM's speed comes from minimal processing and C library backend
- Test splitting from 116s → 0.07s (1657x speedup) proves micro-test value
- LLM hybrid mode (like Marker's `--use_llm`) is worth implementing as optional layer
