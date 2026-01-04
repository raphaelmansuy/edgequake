#!/usr/bin/env python3
"""
Loop 48: Section-by-section comparison to find exactly what content is missing.
"""

import re
from collections import defaultdict

def extract_sections(filepath, name):
    """Extract sections from markdown file."""
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    sections = {}
    lines = content.split('\n')
    
    # Find key sections
    sections['abstract'] = extract_between_markers(lines, ['Abstract'], ['Introduction', '1.'])
    sections['introduction'] = extract_between_markers(lines, ['Introduction', '1.'], ['Related', '2.'])
    sections['related_work'] = extract_between_markers(lines, ['Related', '2.'], ['3.', 'Method'])
    sections['method'] = extract_between_markers(lines, ['Method', '3.'], ['Result', 'Experiment', '4.'])
    sections['results'] = extract_between_markers(lines, ['Result', 'Experiment', '4.'], ['Conclusion', '5.'])
    sections['conclusion'] = extract_between_markers(lines, ['Conclusion', '5.'], ['Reference', 'Acknowledg'])
    sections['references'] = extract_between_markers(lines, ['Reference', 'Acknowledg'], ['999999999'])
    
    return sections

def extract_between_markers(lines, start_markers, end_markers):
    """Extract text between section markers."""
    start_idx = None
    end_idx = len(lines)
    
    # Find start
    for i, line in enumerate(lines):
        for marker in start_markers:
            if marker.lower() in line.lower():
                start_idx = i
                break
        if start_idx is not None:
            break
    
    if start_idx is None:
        return None
    
    # Find end
    for i in range(start_idx + 1, len(lines)):
        for marker in end_markers:
            if marker.lower() in lines[i].lower():
                end_idx = i
                break
        if end_idx < len(lines):
            break
    
    return '\n'.join(lines[start_idx:end_idx])

def count_figures(text):
    """Count figure references."""
    if text is None:
        return 0
    return len(re.findall(r'Fig(?:ure)?\.?\s*\d+', text, re.IGNORECASE))

def count_equations(text):
    """Count equations/formulas."""
    if text is None:
        return 0
    # Look for equation markers or math notation
    latex_count = len(re.findall(r'\$.*?\$|\\[a-z]+{', text))
    parentheses_count = len(re.findall(r'\(\d+\)\s*$', text, re.MULTILINE))
    return max(latex_count, parentheses_count)

def count_references(text):
    """Count citation references."""
    if text is None:
        return 0
    # Look for [1], [2], etc.
    return len(set(re.findall(r'\[(\d+)\]', text)))

def analyze_section(gold_section, current_section, section_name):
    """Detailed comparison of a section."""
    if gold_section is None and current_section is None:
        return f"**{section_name}**: Both missing ❌"
    
    if gold_section is None:
        return f"**{section_name}**: Only in current (unexpected) ⚠️"
    
    if current_section is None:
        return f"**{section_name}**: MISSING from current ❌ CRITICAL"
    
    gold_chars = len(gold_section)
    current_chars = len(current_section)
    retention = 100 * current_chars / gold_chars if gold_chars > 0 else 0
    
    gold_lines = gold_section.count('\n')
    current_lines = current_section.count('\n')
    
    # Detail analysis
    result = f"\n**{section_name}**:\n"
    result += f"  - Gold: {gold_chars:,} chars, {gold_lines} lines\n"
    result += f"  - Current: {current_chars:,} chars, {current_lines} lines\n"
    result += f"  - Retention: {retention:.1f}%\n"
    
    # Figures
    gold_figs = count_figures(gold_section)
    current_figs = count_figures(current_section)
    if gold_figs > 0 or current_figs > 0:
        result += f"  - Figures: {current_figs}/{gold_figs} {'✅' if current_figs >= gold_figs else '⚠️'}\n"
    
    # Equations
    gold_eqs = count_equations(gold_section)
    current_eqs = count_equations(current_section)
    if gold_eqs > 0 or current_eqs > 0:
        result += f"  - Equations: {current_eqs}/{gold_eqs} {'✅' if current_eqs >= gold_eqs * 0.8 else '⚠️'}\n"
    
    # References
    gold_refs = count_references(gold_section)
    current_refs = count_references(current_section)
    if gold_refs > 0 or current_refs > 0:
        result += f"  - Citations: {current_refs}/{gold_refs} {'✅' if current_refs >= gold_refs * 0.8 else '⚠️'}\n"
    
    # Status
    if retention >= 90:
        result += f"  - Status: ✅ GOOD\n"
    elif retention >= 70:
        result += f"  - Status: ⚠️ ACCEPTABLE\n"
    elif retention >= 50:
        result += f"  - Status: ❌ POOR\n"
    else:
        result += f"  - Status: ❌❌ CRITICAL\n"
    
    return result

def main():
    gold_path = "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake/crates/edgequake-pdf/test-data/real_dataset/01_2512.25075v1.gold.md"
    current_path = "/Users/raphaelmansuy/Github/03-working/edgequake/plan_pdf_cli_ooda_loop/loop_47_analysis_current.md"
    
    print("=" * 80)
    print("LOOP 48: SECTION-BY-SECTION COMPARISON")
    print("=" * 80)
    print()
    
    # Extract sections
    print("Extracting sections...")
    gold_sections = extract_sections(gold_path, "gold")
    current_sections = extract_sections(current_path, "current")
    
    print("Analyzing sections...\n")
    print("=" * 80)
    
    # Compare each section
    section_names = ['abstract', 'introduction', 'related_work', 'method', 'results', 'conclusion', 'references']
    
    for section_name in section_names:
        gold_sec = gold_sections.get(section_name)
        current_sec = current_sections.get(section_name)
        result = analyze_section(gold_sec, current_sec, section_name.replace('_', ' ').title())
        print(result)
    
    # Overall summary
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    
    total_gold = sum(len(s) if s else 0 for s in gold_sections.values())
    total_current = sum(len(s) if s else 0 for s in current_sections.values())
    
    print(f"\nTotal Gold: {total_gold:,} chars")
    print(f"Total Current: {total_current:,} chars")
    print(f"Overall Retention: {100 * total_current / total_gold:.1f}%")
    
    # Find worst performers
    print("\n--- Sections Needing Attention ---")
    for section_name in section_names:
        gold_sec = gold_sections.get(section_name)
        current_sec = current_sections.get(section_name)
        
        if gold_sec is None or current_sec is None:
            continue
        
        gold_chars = len(gold_sec)
        current_chars = len(current_sec)
        retention = 100 * current_chars / gold_chars if gold_chars > 0 else 0
        
        if retention < 70:
            print(f"  - {section_name.replace('_', ' ').title()}: {retention:.1f}% ❌")
    
    print("\n--- Next Steps for Loop 49 ---")
    print("  1. Investigate sections with <70% retention")
    print("  2. Check for page ordering issues")
    print("  3. Verify equation/formula extraction")
    print("  4. Validate figure caption completeness")
    print()

if __name__ == "__main__":
    main()
