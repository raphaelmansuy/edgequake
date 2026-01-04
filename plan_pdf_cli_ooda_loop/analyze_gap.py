#!/usr/bin/env python3
"""
Analyze the gap between gold standard and current extraction.
Focus on understanding what content is missing from first principles.
"""

def analyze_files():
    gold_path = "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.gold.md"
    current_path = "/Users/raphaelmansuy/Github/03-working/edgequake/plan_pdf_cli_ooda_loop/loop_47_analysis_current.md"
    
    with open(gold_path, 'r', encoding='utf-8') as f:
        gold_lines = f.readlines()
    
    with open(current_path, 'r', encoding='utf-8') as f:
        current_lines = f.readlines()
    
    print("=" * 80)
    print("FIRST PRINCIPLES ANALYSIS: What's Missing?")
    print("=" * 80)
    print(f"Gold lines: {len(gold_lines)}")
    print(f"Current lines: {len(current_lines)}")
    print(f"Gap: {len(gold_lines) - len(current_lines)} lines ({100 * (len(gold_lines) - len(current_lines)) / len(gold_lines):.1f}%)")
    print()
    
    # Analyze first 100 lines to understand formatting differences
    print("=" * 80)
    print("FIRST 100 LINES - FORMATTING ANALYSIS")
    print("=" * 80)
    
    # Gold first lines
    print("\n--- GOLD (first 50 lines) ---")
    for i, line in enumerate(gold_lines[:50], 1):
        if line.strip():
            print(f"{i:3d}: {repr(line[:80])}")
    
    # Current first lines
    print("\n--- CURRENT (first 50 lines) ---")
    for i, line in enumerate(current_lines[:50], 1):
        if line.strip():
            print(f"{i:3d}: {repr(line[:80])}")
    
    # Character analysis
    print("\n" + "=" * 80)
    print("CHARACTER DISTRIBUTION ANALYSIS")
    print("=" * 80)
    
    # Count single-character lines in gold (likely OCR artifacts)
    gold_single_char = sum(1 for line in gold_lines if len(line.strip()) == 1)
    current_single_char = sum(1 for line in current_lines if len(line.strip()) == 1)
    
    print(f"Gold single-char lines: {gold_single_char} ({100 * gold_single_char / len(gold_lines):.1f}%)")
    print(f"Current single-char lines: {current_single_char} ({100 * current_single_char / len(current_lines):.1f}%)")
    
    # Count blank lines
    gold_blank = sum(1 for line in gold_lines if not line.strip())
    current_blank = sum(1 for line in current_lines if not line.strip())
    
    print(f"Gold blank lines: {gold_blank} ({100 * gold_blank / len(gold_lines):.1f}%)")
    print(f"Current blank lines: {current_blank} ({100 * current_blank / len(current_lines):.1f}%)")
    
    # Average line length
    gold_avg_len = sum(len(line.strip()) for line in gold_lines if line.strip()) / sum(1 for line in gold_lines if line.strip())
    current_avg_len = sum(len(line.strip()) for line in current_lines if line.strip()) / sum(1 for line in current_lines if line.strip())
    
    print(f"Gold avg line length: {gold_avg_len:.1f} chars")
    print(f"Current avg line length: {current_avg_len:.1f} chars")
    
    # Content density
    print("\n" + "=" * 80)
    print("CONTENT DENSITY ANALYSIS")
    print("=" * 80)
    
    gold_total_chars = sum(len(line.strip()) for line in gold_lines)
    current_total_chars = sum(len(line.strip()) for line in current_lines)
    
    print(f"Gold total characters: {gold_total_chars:,}")
    print(f"Current total characters: {current_total_chars:,}")
    print(f"Character retention: {100 * current_total_chars / gold_total_chars:.1f}%")
    
    # Section headers
    print("\n" + "=" * 80)
    print("SECTION STRUCTURE ANALYSIS")
    print("=" * 80)
    
    gold_headers = [line.strip() for line in gold_lines if line.strip().startswith('#')]
    current_headers = [line.strip() for line in current_lines if line.strip().startswith('#')]
    
    print(f"Gold headers: {len(gold_headers)}")
    print(f"Current headers: {len(current_headers)}")
    
    print("\n--- Gold headers (first 20) ---")
    for h in gold_headers[:20]:
        print(f"  {h[:80]}")
    
    print("\n--- Current headers (first 20) ---")
    for h in current_headers[:20]:
        print(f"  {h[:80]}")
    
    # Key insight: Check if gold is just verbose/spaced vs current is compact
    print("\n" + "=" * 80)
    print("KEY INSIGHT: VERBOSITY ANALYSIS")
    print("=" * 80)
    
    # Sample middle sections
    gold_middle = gold_lines[400:500]
    current_middle = current_lines[min(400, len(current_lines)-100):min(500, len(current_lines))]
    
    print("\n--- Gold middle section (lines 400-450) ---")
    print(''.join(gold_middle[:50]))
    
    print("\n--- Current middle section (comparable position) ---")
    print(''.join(current_middle[:50]))
    
    # ROOT CAUSE ANALYSIS
    print("\n" + "=" * 80)
    print("ROOT CAUSE HYPOTHESIS")
    print("=" * 80)
    
    print("""
Based on the analysis:

1. FORMATTING HYPOTHESIS:
   - Gold has many single-character lines (markitdown OCR artifacts)
   - Current uses continuous text (more natural markdown)
   - This could account for ~50% of line count difference

2. CONTENT COMPLETENESS:
   - Character retention: ~75-80% suggests real content is mostly preserved
   - Missing ~20-25% actual content (not just formatting)

3. LIKELY ISSUES:
   - References section may be missing/incomplete
   - Figure captions might be abbreviated
   - Equations/formulas may be simplified
   - Table content might be condensed

4. NEXT STEPS FOR IMPROVEMENT:
   - Don't chase gold's verbose formatting (it's markitdown artifact)
   - Focus on missing substantive content (references, captions, equations)
   - Verify all sections are present (introduction, methods, results, etc.)
   - Check if bibliography/references are extracted
""")

if __name__ == "__main__":
    analyze_files()
