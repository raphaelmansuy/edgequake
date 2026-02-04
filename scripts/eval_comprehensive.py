#!/usr/bin/env python3
"""
Comprehensive PDF-to-Markdown Evaluation Metrics

DESIGN RATIONALE (First Principles Analysis):

The current word-set F1 metric has critical limitations:
1. SET ignores word ORDER → scrambled text scores same as correct text
2. SET ignores DUPLICATES → repeated words (common in academic papers) are lost
3. Strips markdown → doesn't verify headers, bold, italic preservation
4. Single threshold (len > 1) may filter important content

This script implements a multi-dimensional evaluation suite:

DIMENSION 1: CONTENT ACCURACY (What words are present?)
- Word F1 (bag of words): Current metric - captures vocabulary coverage
- Unigram Precision/Recall: Standard NLP metric

DIMENSION 2: ORDER PRESERVATION (Are words in correct sequence?)
- ROUGE-L (LCS): Longest Common Subsequence - captures reading order
- BLEU-4: N-gram precision with brevity penalty - captures phrase structure
- Word Levenshtein: Edit distance normalized - captures insertions/deletions

DIMENSION 3: STRUCTURAL FIDELITY (Is document structure preserved?)
- Heading Count Match: Number of # markers should match
- Paragraph Count Match: Number of blank-line-separated blocks
- Line Count Ratio: Total lines comparison

DIMENSION 4: FORMATTING FIDELITY (Is markdown formatting preserved?)
- Bold Marker Count: Number of **text** patterns
- Italic Marker Count: Number of *text* or _text_ patterns
- List Item Count: Number of - or * list markers

COMPOSITE SCORE:
- Weighted harmonic mean prioritizing order (ROUGE-L most important)
- Formula: Quality = 0.4*ROUGE_L + 0.3*Word_F1 + 0.2*Structural + 0.1*Formatting

Usage:
  python3 scripts/eval_comprehensive.py
  python3 scripts/eval_comprehensive.py --verbose
  python3 scripts/eval_comprehensive.py --file AlphaEvolve
"""

import argparse
import os
import re
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path


@dataclass
class MetricsResult:
    """All computed metrics for a single file comparison."""

    file_name: str

    # Dimension 1: Content Accuracy
    word_precision: float
    word_recall: float
    word_f1: float

    # Dimension 2: Order Preservation
    rouge_l: float  # LCS-based
    bleu_4: float  # 4-gram precision with brevity penalty
    word_levenshtein: float  # Normalized edit distance

    # Dimension 3: Structural Fidelity
    heading_match: float  # Jaccard similarity of heading counts by level
    paragraph_match: float  # Ratio of paragraph counts
    line_ratio: float  # Ratio of line counts

    # Dimension 4: Formatting Fidelity
    bold_match: float  # Ratio of bold markers
    italic_match: float  # Ratio of italic markers
    list_match: float  # Ratio of list markers

    # Composite Scores
    order_score: float  # Combined order preservation
    structure_score: float  # Combined structural fidelity
    format_score: float  # Combined formatting fidelity
    quality_score: float  # Overall quality (weighted harmonic mean)


def tokenize(text: str) -> list[str]:
    """Tokenize text into words, preserving order."""
    # Normalize: lowercase, strip markdown markers from individual words
    words = text.lower().split()
    # Remove leading/trailing punctuation but keep word content
    words = [re.sub(r"^[*_`#\[\]()]+|[*_`#\[\]()]+$", "", w) for w in words]
    # Filter empty and single-char words
    words = [w for w in words if w and len(w) > 1]
    return words


def compute_word_f1(
    gold_words: list[str], extracted_words: list[str]
) -> tuple[float, float, float]:
    """
    Compute word-level precision, recall, F1 using BAG OF WORDS (multiset).

    Unlike the original SET-based approach, this uses Counter (multiset)
    to properly count duplicate words.
    """
    gold_counts = Counter(gold_words)
    extracted_counts = Counter(extracted_words)

    # True positives: min count for each word
    true_positive = sum((gold_counts & extracted_counts).values())

    precision = (
        true_positive / sum(extracted_counts.values()) if extracted_counts else 0
    )
    recall = true_positive / sum(gold_counts.values()) if gold_counts else 0

    if precision + recall == 0:
        f1 = 0
    else:
        f1 = 2 * precision * recall / (precision + recall)

    return precision, recall, f1


def compute_lcs_length(a: list[str], b: list[str]) -> int:
    """Compute Longest Common Subsequence length using dynamic programming."""
    m, n = len(a), len(b)
    if m == 0 or n == 0:
        return 0

    # Space optimization: only keep two rows
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
    """
    Compute ROUGE-L F1 score based on Longest Common Subsequence.

    ROUGE-L measures the longest in-order match between texts.
    This captures reading order preservation which SET-based F1 cannot.

    Formula:
      Precision_LCS = LCS / len(extracted)
      Recall_LCS = LCS / len(gold)
      ROUGE-L = F1(Precision_LCS, Recall_LCS)
    """
    if not gold_words or not extracted_words:
        return 0.0

    lcs_len = compute_lcs_length(gold_words, extracted_words)

    precision = lcs_len / len(extracted_words)
    recall = lcs_len / len(gold_words)

    if precision + recall == 0:
        return 0.0

    return 2 * precision * recall / (precision + recall)


def compute_ngram_precision(
    gold_words: list[str], extracted_words: list[str], n: int
) -> float:
    """Compute n-gram precision for BLEU calculation."""
    if len(extracted_words) < n:
        return 0.0

    # Build n-gram counts
    def get_ngrams(words, n):
        return [tuple(words[i : i + n]) for i in range(len(words) - n + 1)]

    extracted_ngrams = Counter(get_ngrams(extracted_words, n))
    gold_ngrams = Counter(get_ngrams(gold_words, n))

    # Clipped counts: min of extracted count and max gold count
    clipped = sum((extracted_ngrams & gold_ngrams).values())
    total = sum(extracted_ngrams.values())

    return clipped / total if total > 0 else 0.0


def compute_bleu_4(gold_words: list[str], extracted_words: list[str]) -> float:
    """
    Compute BLEU-4 score with brevity penalty.

    BLEU uses geometric mean of n-gram precisions (1-4) with brevity penalty.
    Captures phrase structure and fluency.
    """
    import math

    if not extracted_words or not gold_words:
        return 0.0

    # Compute n-gram precisions for n=1,2,3,4
    precisions = []
    for n in range(1, 5):
        p = compute_ngram_precision(gold_words, extracted_words, n)
        # Add smoothing to avoid zero
        if p == 0:
            p = 0.01  # Smoothing
        precisions.append(p)

    # Geometric mean of precisions with uniform weights
    log_precision = sum(math.log(p) for p in precisions) / 4

    # Brevity penalty
    c = len(extracted_words)
    r = len(gold_words)

    if c >= r:
        bp = 1.0
    else:
        bp = math.exp(1 - r / c)

    return bp * math.exp(log_precision)


def compute_word_levenshtein(
    gold_words: list[str], extracted_words: list[str]
) -> float:
    """
    Compute normalized word-level Levenshtein distance.

    Returns 1 - (edit_distance / max_length) so higher is better.
    Uses dynamic programming for efficiency.
    """
    m, n = len(gold_words), len(extracted_words)

    if m == 0 and n == 0:
        return 1.0
    if m == 0 or n == 0:
        return 0.0

    # DP with space optimization
    prev = list(range(n + 1))
    curr = [0] * (n + 1)

    for i in range(1, m + 1):
        curr[0] = i
        for j in range(1, n + 1):
            if gold_words[i - 1] == extracted_words[j - 1]:
                curr[j] = prev[j - 1]
            else:
                curr[j] = 1 + min(prev[j], curr[j - 1], prev[j - 1])
        prev, curr = curr, prev

    edit_distance = prev[n]
    max_len = max(m, n)

    return 1.0 - (edit_distance / max_len)


def count_headings(text: str) -> dict[int, int]:
    """Count headings by level (1-6)."""
    counts = {}
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("#"):
            # Count leading #
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

    # Compare by level
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
    """Compute ratio of paragraph counts (bounded to [0,1])."""
    gold_count = count_paragraphs(gold_text)
    extracted_count = count_paragraphs(extracted_text)

    if gold_count == 0 and extracted_count == 0:
        return 1.0
    if gold_count == 0 or extracted_count == 0:
        return 0.0

    ratio = min(gold_count, extracted_count) / max(gold_count, extracted_count)
    return ratio


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
    """Count *italic* or _italic_ patterns (excluding bold)."""
    # Remove bold first to avoid counting **text** as italic
    text = re.sub(r"\*\*[^*]+\*\*", "", text)
    single_star = len(re.findall(r"\*[^*]+\*", text))
    underscore = len(re.findall(r"_[^_]+_", text))
    return single_star + underscore


def count_list_markers(text: str) -> int:
    """Count list items (lines starting with - or *)."""
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


def run_pipeline(pdf_path: Path, lib_path: Path) -> str | None:
    """Run the Rust pipeline and capture output."""
    env = os.environ.copy()
    env["PDFIUM_DYNAMIC_LIB_PATH"] = str(lib_path)

    try:
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--features",
                "pdfium",
                "-p",
                "edgequake-pdf",
                "--example",
                "convert_pdf_full",
                "--",
                str(pdf_path),
            ],
            cwd=Path(__file__).parent.parent / "edgequake",
            capture_output=True,
            text=True,
            timeout=120,
            env=env,
        )

        if result.returncode != 0:
            return None

        return result.stdout

    except subprocess.TimeoutExpired:
        return None
    except Exception:
        return None


def compute_all_metrics(
    gold_text: str, extracted_text: str, file_name: str
) -> MetricsResult:
    """Compute all metrics for a gold/extracted pair."""

    # Tokenize
    gold_words = tokenize(gold_text)
    extracted_words = tokenize(extracted_text)

    # Dimension 1: Content Accuracy
    word_precision, word_recall, word_f1 = compute_word_f1(gold_words, extracted_words)

    # Dimension 2: Order Preservation
    rouge_l = compute_rouge_l(gold_words, extracted_words)
    bleu_4 = compute_bleu_4(gold_words, extracted_words)
    word_lev = compute_word_levenshtein(gold_words, extracted_words)

    # Dimension 3: Structural Fidelity
    heading_match = compute_heading_match(gold_text, extracted_text)
    paragraph_match = compute_paragraph_match(gold_text, extracted_text)
    line_ratio = compute_line_ratio(gold_text, extracted_text)

    # Dimension 4: Formatting Fidelity
    bold_match = compute_format_match(
        count_bold_markers(gold_text), count_bold_markers(extracted_text)
    )
    italic_match = compute_format_match(
        count_italic_markers(gold_text), count_italic_markers(extracted_text)
    )
    list_match = compute_format_match(
        count_list_markers(gold_text), count_list_markers(extracted_text)
    )

    # Composite Scores
    order_score = 0.5 * rouge_l + 0.3 * bleu_4 + 0.2 * word_lev
    structure_score = 0.4 * heading_match + 0.3 * paragraph_match + 0.3 * line_ratio
    format_score = 0.4 * bold_match + 0.4 * italic_match + 0.2 * list_match

    # Quality Score: Weighted combination prioritizing order
    # ROUGE-L is most important because it captures reading order
    quality_score = (
        0.40 * rouge_l  # Order preservation (most critical)
        + 0.30 * word_f1  # Content accuracy
        + 0.15 * structure_score  # Document structure
        + 0.10 * format_score  # Markdown formatting
        + 0.05 * bleu_4  # Phrase structure
    )

    return MetricsResult(
        file_name=file_name,
        word_precision=word_precision,
        word_recall=word_recall,
        word_f1=word_f1,
        rouge_l=rouge_l,
        bleu_4=bleu_4,
        word_levenshtein=word_lev,
        heading_match=heading_match,
        paragraph_match=paragraph_match,
        line_ratio=line_ratio,
        bold_match=bold_match,
        italic_match=italic_match,
        list_match=list_match,
        order_score=order_score,
        structure_score=structure_score,
        format_score=format_score,
        quality_score=quality_score,
    )


def print_metrics(m: MetricsResult, verbose: bool = False):
    """Print metrics for a single file."""
    print(f"📄 {m.file_name}")
    print(f"   QUALITY: {m.quality_score:.3f}  (target: ≥0.95)")
    print(f"   ├─ ROUGE-L:    {m.rouge_l:.3f}  (order preservation)")
    print(f"   ├─ Word F1:    {m.word_f1:.3f}  (content accuracy)")
    print(f"   ├─ Structure:  {m.structure_score:.3f}  (document structure)")
    print(f"   └─ Format:     {m.format_score:.3f}  (markdown fidelity)")

    if verbose:
        print(f"\n   Dimension 1 - Content Accuracy:")
        print(
            f"     Precision: {m.word_precision:.3f}, Recall: {m.word_recall:.3f}, F1: {m.word_f1:.3f}"
        )
        print(f"\n   Dimension 2 - Order Preservation:")
        print(
            f"     ROUGE-L: {m.rouge_l:.3f}, BLEU-4: {m.bleu_4:.3f}, Levenshtein: {m.word_levenshtein:.3f}"
        )
        print(f"\n   Dimension 3 - Structural Fidelity:")
        print(
            f"     Headings: {m.heading_match:.3f}, Paragraphs: {m.paragraph_match:.3f}, Lines: {m.line_ratio:.3f}"
        )
        print(f"\n   Dimension 4 - Formatting Fidelity:")
        print(
            f"     Bold: {m.bold_match:.3f}, Italic: {m.italic_match:.3f}, Lists: {m.list_match:.3f}"
        )
    print()


def main():
    parser = argparse.ArgumentParser(
        description="Comprehensive PDF-to-Markdown evaluation"
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Show detailed metrics"
    )
    parser.add_argument(
        "--file", "-f", type=str, help="Evaluate single file (stem name)"
    )
    args = parser.parse_args()

    # Paths
    script_dir = Path(__file__).parent.parent
    test_data = script_dir / "edgequake/crates/edgequake-pdf/test-data/real_dataset"
    lib_path = script_dir / "edgequake/crates/edgequake-pdf/lib/lib/libpdfium.dylib"

    if not lib_path.exists():
        print(f"ERROR: libpdfium.dylib not found at {lib_path}")
        sys.exit(1)

    # Find gold standard pairs
    pdf_files = list(test_data.glob("*.pdf"))
    if args.file:
        pdf_files = [p for p in pdf_files if args.file in p.stem]

    results: list[MetricsResult] = []

    print(f"\n{'='*70}")
    print("Comprehensive PDF-to-Markdown Quality Evaluation")
    print(f"{'='*70}")
    print("\nMetrics Design:")
    print("  • ROUGE-L: Longest Common Subsequence (captures reading ORDER)")
    print("  • Word F1: Bag-of-words with multiset (captures CONTENT)")
    print("  • Structure: Headings, paragraphs, lines (captures LAYOUT)")
    print("  • Format: Bold, italic, lists (captures MARKDOWN)")
    print(f"{'='*70}\n")

    for pdf_path in sorted(pdf_files):
        stem = pdf_path.stem
        gold_path = test_data / f"{stem}.pymupdf.gold.md"

        if not gold_path.exists():
            print(f"⏭ {stem}: No pymupdf gold standard")
            continue

        # Read gold
        gold_text = gold_path.read_text()

        # Run pipeline
        extracted = run_pipeline(pdf_path, lib_path)
        if not extracted:
            print(f"❌ {stem}: Failed to extract")
            continue

        # Compute metrics
        metrics = compute_all_metrics(gold_text, extracted, stem)
        results.append(metrics)
        print_metrics(metrics, verbose=args.verbose)

    # Summary
    if results:
        avg_quality = sum(r.quality_score for r in results) / len(results)
        avg_rouge_l = sum(r.rouge_l for r in results) / len(results)
        avg_word_f1 = sum(r.word_f1 for r in results) / len(results)
        avg_structure = sum(r.structure_score for r in results) / len(results)
        avg_format = sum(r.format_score for r in results) / len(results)

        print(f"{'='*70}")
        print(f"SUMMARY ({len(results)} files)")
        print(f"{'='*70}")
        print(
            f"  Average QUALITY:   {avg_quality:.3f}  (target: ≥0.95, gap: {0.95 - avg_quality:+.3f})"
        )
        print(f"  Average ROUGE-L:   {avg_rouge_l:.3f}  (order preservation)")
        print(f"  Average Word F1:   {avg_word_f1:.3f}  (content accuracy)")
        print(f"  Average Structure: {avg_structure:.3f}  (document structure)")
        print(f"  Average Format:    {avg_format:.3f}  (markdown fidelity)")
        print(f"{'='*70}")

        # Comparison table
        print("\nPer-file Comparison (sorted by quality):")
        print(
            f"{'File':<30} {'Quality':>8} {'ROUGE-L':>8} {'Word F1':>8} {'Struct':>8} {'Format':>8}"
        )
        print("-" * 70)
        for r in sorted(results, key=lambda x: x.quality_score, reverse=True):
            print(
                f"{r.file_name:<30} {r.quality_score:>8.3f} {r.rouge_l:>8.3f} {r.word_f1:>8.3f} {r.structure_score:>8.3f} {r.format_score:>8.3f}"
            )
        print()
    else:
        print("\nNo files evaluated.")


if __name__ == "__main__":
    main()
