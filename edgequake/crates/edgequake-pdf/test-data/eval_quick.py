#!/usr/bin/env python3
"""Quick quality evaluation matching spec formula"""
import re
import sys
from collections import Counter


def ngrams(words, n):
    return [tuple(words[i : i + n]) for i in range(len(words) - n + 1)]


def word_tokenize(text):
    return re.findall(r"\w+", text.lower())


def rouge_l(ref_words, hyp_words):
    """Compute ROUGE-L (LCS-based)"""
    if not ref_words or not hyp_words:
        return 0.0

    m, n = len(ref_words), len(hyp_words)
    # LCS via DP
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if ref_words[i - 1] == hyp_words[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    lcs = dp[m][n]
    prec = lcs / n if n else 0
    rec = lcs / m if m else 0
    if prec + rec == 0:
        return 0.0
    return 2 * prec * rec / (prec + rec)


def bleu4(ref_words, hyp_words):
    """Simplified BLEU-4"""
    if len(hyp_words) < 4:
        return 0.0
    scores = []
    for n in range(1, 5):
        ref_ng = Counter(ngrams(ref_words, n))
        hyp_ng = Counter(ngrams(hyp_words, n))
        overlap = sum((min(ref_ng[ng], hyp_ng[ng]) for ng in hyp_ng))
        total = sum(hyp_ng.values())
        scores.append(overlap / total if total else 0)
    from math import exp, log

    if 0 in scores:
        return 0.0
    return exp(sum(log(s) for s in scores) / 4)


def word_f1(ref_words, hyp_words):
    ref_set = set(ref_words)
    hyp_set = set(hyp_words)
    if not ref_set or not hyp_set:
        return 0.0
    common = ref_set & hyp_set
    prec = len(common) / len(hyp_set)
    rec = len(common) / len(ref_set)
    if prec + rec == 0:
        return 0.0
    return 2 * prec * rec / (prec + rec)


def count_headers(text):
    return len(re.findall(r"^#{1,6}\s", text, re.MULTILINE))


def count_bold(text):
    return len(re.findall(r"\*\*[^*]+\*\*", text))


def count_italic(text):
    # Match *text* but not **text**
    return len(re.findall(r"(?<!\*)\*(?!\*)[^*]+\*(?!\*)", text))


def count_lists(text):
    return len(re.findall(r"^[\s]*[-*+]\s|^[\s]*\d+\.\s", text, re.MULTILINE))


def format_score(gold, ours):
    """Format F1 score between gold and ours format counts"""
    gold_h = count_headers(gold)
    ours_h = count_headers(ours)
    gold_b = count_bold(gold)
    ours_b = count_bold(ours)
    gold_i = count_italic(gold)
    ours_i = count_italic(ours)
    gold_l = count_lists(gold)
    ours_l = count_lists(ours)

    print(f"  Headers: {ours_h} vs {gold_h} gold")
    print(f"  Bold:    {ours_b} vs {gold_b} gold")
    print(f"  Italic:  {ours_i} vs {gold_i} gold")
    print(f"  Lists:   {ours_l} vs {gold_l} gold")

    def f1(g, o):
        if g == 0 and o == 0:
            return 1.0
        if g == 0 or o == 0:
            return 0.0
        prec = min(g, o) / o
        rec = min(g, o) / g
        return 2 * prec * rec / (prec + rec) if prec + rec else 0

    h_f1 = f1(gold_h, ours_h)
    b_f1 = f1(gold_b, ours_b)
    i_f1 = f1(gold_i, ours_i)
    l_f1 = f1(gold_l, ours_l)

    return (h_f1 + b_f1 + i_f1 + l_f1) / 4


def structure_score(gold, ours):
    """Simple structure score based on line count similarity"""
    gold_lines = len([l for l in gold.split("\n") if l.strip()])
    ours_lines = len([l for l in ours.split("\n") if l.strip()])
    if gold_lines == 0:
        return 1.0 if ours_lines == 0 else 0.0
    ratio = min(gold_lines, ours_lines) / max(gold_lines, ours_lines)
    return ratio


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: eval_quick.py <ours.md> <gold.md>")
        sys.exit(1)

    ours = open(sys.argv[1]).read()
    gold = open(sys.argv[2]).read()

    ours_words = word_tokenize(ours)
    gold_words = word_tokenize(gold)

    rl = rouge_l(gold_words, ours_words)
    wf1 = word_f1(gold_words, ours_words)
    struct = structure_score(gold, ours)
    fmt = format_score(gold, ours)
    bl4 = bleu4(gold_words, ours_words)

    quality = 0.40 * rl + 0.30 * wf1 + 0.15 * struct + 0.10 * fmt + 0.05 * bl4

    print(f"\n=== Quality Evaluation ===")
    print(f"ROUGE-L:   {rl:.3f} (x0.40 = {0.40*rl:.3f})")
    print(f"Word F1:   {wf1:.3f} (x0.30 = {0.30*wf1:.3f})")
    print(f"Structure: {struct:.3f} (x0.15 = {0.15*struct:.3f})")
    print(f"Format:    {fmt:.3f} (x0.10 = {0.10*fmt:.3f})")
    print(f"BLEU-4:    {bl4:.3f} (x0.05 = {0.05*bl4:.3f})")
    print(f"-----------------------------")
    print(f"QUALITY:   {quality:.3f}")
    print(f"Target:    >=0.950")
    print(f"Gap:       {max(0, 0.95 - quality):.3f}")
