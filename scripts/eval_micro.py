#!/usr/bin/env python3
"""
Micro-evaluation script for fast iteration during OODA loops.

This script evaluates a SINGLE file quickly to provide feedback on changes.
Use this instead of eval_comprehensive.py for faster iteration.

Usage:
  python3 scripts/eval_micro.py                    # Evaluate one file (2900_Goyal_et_al by default)
  python3 scripts/eval_micro.py --file AlphaEvolve # Evaluate specific file
  python3 scripts/eval_micro.py --all              # Evaluate all files (slower)
"""

import argparse
import re
import sys
from collections import Counter
from pathlib import Path


def tokenize(text: str) -> list[str]:
    """Tokenize text into words, preserving order."""
    words = text.lower().split()
    words = [re.sub(r"^[*_`#\[\]()]+|[*_`#\[\]()]+$", "", w) for w in words]
    words = [w for w in words if w and len(w) > 1]
    return words


def compute_word_f1(gold_words: list[str], extracted_words: list[str]) -> float:
    """Compute word-level F1 using BAG OF WORDS."""
    gold_counts = Counter(gold_words)
    extracted_counts = Counter(extracted_words)
    true_positive = sum((gold_counts & extracted_counts).values())
    precision = (
        true_positive / sum(extracted_counts.values()) if extracted_counts else 0
    )
    recall = true_positive / sum(gold_counts.values()) if gold_counts else 0
    if precision + recall == 0:
        return 0.0
    return 2 * precision * recall / (precision + recall)


def compute_lcs_length(a: list[str], b: list[str]) -> int:
    """Compute LCS length using DP."""
    m, n = len(a), len(b)
    if m == 0 or n == 0:
        return 0
    prev = [0] * (n + 1)
    curr = [0] * (n + 1)
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if a[i - 1] == b[j - 1]:
                curr[j] = prev[j - 1] + 1
            else:
                curr[j] = max(prev[j], curr[j - 1])
        prev, curr = curr, prev
    return prev[n]


def compute_rouge_l(gold_words: list[str], extracted_words: list[str]) -> float:
    """Compute ROUGE-L F1 score."""
    if not gold_words or not extracted_words:
        return 0.0
    lcs_len = compute_lcs_length(gold_words, extracted_words)
    precision = lcs_len / len(extracted_words)
    recall = lcs_len / len(gold_words)
    if precision + recall == 0:
        return 0.0
    return 2 * precision * recall / (precision + recall)


def count_headings(text: str) -> dict[int, int]:
    """Count headings by level (1-6)."""
    counts = {}
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("#"):
            level = 0
            for c in line:
                if c == "#":
                    level += 1
                else:
                    break
            if 1 <= level <= 6:
                counts[level] = counts.get(level, 0) + 1
    return counts


def compute_heading_match(gold_text: str, extracted_text: str) -> float:
    """Compute Jaccard similarity of heading counts."""
    gold_headings = count_headings(gold_text)
    extracted_headings = count_headings(extracted_text)
    if not gold_headings and not extracted_headings:
        return 1.0
    if not gold_headings or not extracted_headings:
        return 0.0
    all_levels = set(gold_headings.keys()) | set(extracted_headings.keys())
    intersection = sum(
        min(gold_headings.get(l, 0), extracted_headings.get(l, 0)) for l in all_levels
    )
    union = sum(
        max(gold_headings.get(l, 0), extracted_headings.get(l, 0)) for l in all_levels
    )
    return intersection / union if union > 0 else 0.0


def count_paragraphs(text: str) -> int:
    """Count paragraphs (separated by blank lines)."""
    paragraphs = re.split(r"\n\s*\n", text.strip())
    return len([p for p in paragraphs if p.strip()])


def compute_paragraph_match(gold_text: str, extracted_text: str) -> float:
    """Compute ratio of paragraph counts."""
    gold_count = count_paragraphs(gold_text)
    extracted_count = count_paragraphs(extracted_text)
    if gold_count == 0 and extracted_count == 0:
        return 1.0
    if gold_count == 0 or extracted_count == 0:
        return 0.0
    return min(gold_count, extracted_count) / max(gold_count, extracted_count)


def compute_line_ratio(gold_text: str, extracted_text: str) -> float:
    """Compute ratio of line counts."""
    gold_lines = len([l for l in gold_text.split("\n") if l.strip()])
    extracted_lines = len([l for l in extracted_text.split("\n") if l.strip()])
    if gold_lines == 0 and extracted_lines == 0:
        return 1.0
    if gold_lines == 0 or extracted_lines == 0:
        return 0.0
    return min(gold_lines, extracted_lines) / max(gold_lines, extracted_lines)


def count_bold_markers(text: str) -> int:
    """Count **bold** patterns."""
    return len(re.findall(r"\*\*[^*]+\*\*", text))


def count_italic_markers(text: str) -> int:
    """Count *italic* or _italic_ patterns."""
    text = re.sub(r"\*\*[^*]+\*\*", "", text)
    single_star = len(re.findall(r"\*[^*]+\*", text))
    underscore = len(re.findall(r"_[^_]+_", text))
    return single_star + underscore


def count_list_markers(text: str) -> int:
    """Count list items."""
    count = 0
    for line in text.split("\n"):
        line = line.strip()
        if re.match(r"^[-*]\s", line):
            count += 1
    return count


def compute_format_match(gold_count: int, extracted_count: int) -> float:
    """Compute format marker match ratio."""
    if gold_count == 0 and extracted_count == 0:
        return 1.0
    if gold_count == 0 or extracted_count == 0:
        return 0.0
    return min(gold_count, extracted_count) / max(gold_count, extracted_count)


def evaluate_file(gold_text: str, extracted_text: str, file_name: str) -> dict:
    """Evaluate a single file and return metrics."""
    gold_words = tokenize(gold_text)
    extracted_words = tokenize(extracted_text)

    word_f1 = compute_word_f1(gold_words, extracted_words)
    rouge_l = compute_rouge_l(gold_words, extracted_words)

    heading_match = compute_heading_match(gold_text, extracted_text)
    paragraph_match = compute_paragraph_match(gold_text, extracted_text)
    line_ratio = compute_line_ratio(gold_text, extracted_text)
    structure_score = 0.4 * heading_match + 0.3 * paragraph_match + 0.3 * line_ratio

    bold_match = compute_format_match(
        count_bold_markers(gold_text), count_bold_markers(extracted_text)
    )
    italic_match = compute_format_match(
        count_italic_markers(gold_text), count_italic_markers(extracted_text)
    )
    list_match = compute_format_match(
        count_list_markers(gold_text), count_list_markers(extracted_text)
    )
    format_score = 0.4 * bold_match + 0.4 * italic_match + 0.2 * list_match

    quality_score = (
        0.40 * rouge_l
        + 0.30 * word_f1
        + 0.15 * structure_score
        + 0.10 * format_score
        + 0.05 * 0.5
    )

    return {
        "file_name": file_name,
        "quality": quality_score,
        "rouge_l": rouge_l,
        "word_f1": word_f1,
        "structure": structure_score,
        "format": format_score,
        "headings": (
            sum(count_headings(gold_text).values()),
            sum(count_headings(extracted_text).values()),
        ),
        "bold": (count_bold_markers(gold_text), count_bold_markers(extracted_text)),
        "italic": (
            count_italic_markers(gold_text),
            count_italic_markers(extracted_text),
        ),
        "lists": (count_list_markers(gold_text), count_list_markers(extracted_text)),
    }


def main():
    parser = argparse.ArgumentParser(description="Fast micro-evaluation for OODA loops")
    parser.add_argument(
        "--file",
        "-f",
        type=str,
        default="2900_Goyal_et_al",
        help="File stem to evaluate",
    )
    parser.add_argument("--all", "-a", action="store_true", help="Evaluate all files")
    args = parser.parse_args()

    script_dir = Path(__file__).parent.parent
    test_data_dir = (
        script_dir
        / "edgequake"
        / "crates"
        / "edgequake-pdf"
        / "test-data"
        / "real_dataset"
    )

    if args.all:
        pdf_files = sorted(test_data_dir.glob("*.pdf"))
    else:
        pdf_files = [test_data_dir / f"{args.file}.pdf"]

    results = []
    for pdf_path in pdf_files:
        stem = pdf_path.stem
        gold_path = test_data_dir / f"{stem}.pymupdf.gold.md"
        extracted_path = test_data_dir / f"{stem}.md"

        if not gold_path.exists():
            print(f"⚠️  Gold file not found: {gold_path.name}")
            continue
        if not extracted_path.exists():
            print(f"⚠️  Extracted file not found: {extracted_path.name}")
            continue

        gold_text = gold_path.read_text()
        extracted_text = extracted_path.read_text()

        result = evaluate_file(gold_text, extracted_text, stem)
        results.append(result)

        print(f"\n📄 {result['file_name']}")
        print(f"   QUALITY: {result['quality']:.3f}  (target: ≥0.95)")
        print(f"   ├─ ROUGE-L:    {result['rouge_l']:.3f}")
        print(f"   ├─ Word F1:    {result['word_f1']:.3f}")
        print(
            f"   ├─ Structure:  {result['structure']:.3f}  (headings: gold={result['headings'][0]}, ours={result['headings'][1]})"
        )
        print(
            f"   └─ Format:     {result['format']:.3f}  (bold: {result['bold'][0]}/{result['bold'][1]}, italic: {result['italic'][0]}/{result['italic'][1]}, lists: {result['lists'][0]}/{result['lists'][1]})"
        )

    if len(results) > 1:
        avg_quality = sum(r["quality"] for r in results) / len(results)
        avg_rouge = sum(r["rouge_l"] for r in results) / len(results)
        avg_word = sum(r["word_f1"] for r in results) / len(results)
        avg_struct = sum(r["structure"] for r in results) / len(results)
        avg_format = sum(r["format"] for r in results) / len(results)
        print(f"\n{'='*60}")
        print(f"AVERAGE ({len(results)} files)")
        print(
            f"   QUALITY: {avg_quality:.3f}  ROUGE-L: {avg_rouge:.3f}  Word F1: {avg_word:.3f}"
        )
        print(f"   Structure: {avg_struct:.3f}  Format: {avg_format:.3f}")
        print(f"{'='*60}")


if __name__ == "__main__":
    main()
