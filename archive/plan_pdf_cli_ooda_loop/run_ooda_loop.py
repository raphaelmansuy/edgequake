#!/usr/bin/env python3
"""
OODA Loop Executor - Run tests, compare with gold, identify issues.

This script:
1. OBSERVE: Convert all PDFs to markdown using edgequake-pdf
2. ORIENT: Compare with gold markdown and calculate diff scores
3. DECIDE: Categorize and prioritize issues
4. ACT: Generate fix recommendations

Runs continuously for 30 OODA loops until all issues are fixed.
"""

import difflib
import json
import os
import re
import subprocess
import sys
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

# Paths
BASE_DIR = Path(__file__).parent
PDF_DIR = BASE_DIR / "01-generated-pdfs-v2"
GOLD_DIR = BASE_DIR / "02-gold-markdown"
CONVERTED_DIR = BASE_DIR / "03-converted-v2"
OBSERVE_DIR = BASE_DIR / "04-observe-v2"
ORIENT_DIR = BASE_DIR / "05-orient-v2"
DECIDE_DIR = BASE_DIR / "06-decide-v2"
ACT_DIR = BASE_DIR / "07-act-v2"

for d in [CONVERTED_DIR, OBSERVE_DIR, ORIENT_DIR, DECIDE_DIR, ACT_DIR]:
    d.mkdir(exist_ok=True)

# edgequake-pdf binary
EDGEQUAKE_PDF = (
    Path(__file__).parent.parent / "edgequake" / "target" / "release" / "edgequake-pdf"
)


@dataclass
class Issue:
    """Represents a detected issue."""

    id: str
    category: str
    severity: str  # critical, major, minor
    test_case: str
    generator: str
    description: str
    expected: str
    actual: str
    line_number: Optional[int] = None


@dataclass
class TestResult:
    """Result of a single test conversion."""

    test_case: str
    generator: str
    pdf_path: str
    gold_path: str
    converted_path: str
    similarity: float
    issues: List[Issue]
    diff_summary: str


def ensure_binary():
    """Ensure edgequake-pdf binary is built."""
    if not EDGEQUAKE_PDF.exists():
        print("📦 Building edgequake-pdf...")
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "edgequake-pdf"],
            cwd=BASE_DIR.parent / "edgequake",
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"❌ Build failed: {result.stderr}")
            sys.exit(1)
        print("✅ Build complete")
    return EDGEQUAKE_PDF


def convert_pdf(pdf_path: Path, output_path: Path, use_vision: bool = False) -> bool:
    """Convert PDF to markdown using edgequake-pdf."""
    binary = ensure_binary()

    cmd = [str(binary), "convert", "-i", str(pdf_path), "-o", str(output_path)]
    if use_vision:
        cmd.append("--vision")

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        return False
    except Exception as e:
        print(f"  ⚠️  Error: {e}")
        return False


def calculate_similarity(text1: str, text2: str) -> float:
    """Calculate similarity ratio between two texts."""
    # Normalize whitespace and line endings
    text1 = "\n".join(line.strip() for line in text1.strip().split("\n"))
    text2 = "\n".join(line.strip() for line in text2.strip().split("\n"))

    matcher = difflib.SequenceMatcher(None, text1, text2)
    return matcher.ratio() * 100


def generate_diff(gold: str, converted: str) -> str:
    """Generate unified diff between gold and converted."""
    gold_lines = gold.strip().split("\n")
    converted_lines = converted.strip().split("\n")

    diff = difflib.unified_diff(
        gold_lines, converted_lines, fromfile="gold", tofile="converted", lineterm=""
    )
    return "\n".join(diff)


def categorize_issue(expected: str, actual: str) -> tuple:
    """Categorize an issue based on the difference."""

    # Heading issues
    if expected.startswith("#") and not actual.startswith("#"):
        return ("heading", "major", "Heading not detected")
    if expected.startswith("#") and actual.startswith("#"):
        exp_level = len(expected.split()[0])
        act_level = len(actual.split()[0]) if actual.split() else 0
        if exp_level != act_level:
            return (
                "heading",
                "major",
                f"Wrong heading level: expected H{exp_level}, got H{act_level}",
            )

    # List issues
    if expected.strip().startswith("- ") and not actual.strip().startswith("- "):
        return ("list", "major", "List item not detected")
    if expected.strip().startswith("1. ") and not actual.strip().startswith("1. "):
        return ("list", "major", "Numbered list not detected")

    # Table issues
    if "|" in expected and "|" not in actual:
        return ("table", "major", "Table not detected")

    # Code block issues
    if expected.strip().startswith("```") and not actual.strip().startswith("```"):
        return ("code", "major", "Code block not detected")

    # Bold/italic issues
    if "**" in expected and "**" not in actual:
        return ("formatting", "minor", "Bold not detected")
    if "*" in expected and "*" not in actual:
        return ("formatting", "minor", "Italic not detected")

    # Inline code issues
    if "`" in expected and "`" not in actual:
        return ("code", "minor", "Inline code not detected")

    # Link issues
    if "[" in expected and "](" in expected and "[" not in actual:
        return ("link", "minor", "Link not detected")

    # Whitespace issues
    if expected.strip() == actual.strip():
        return ("whitespace", "minor", "Whitespace difference")

    # Generic text difference
    return ("text", "major", "Text content differs")


def analyze_diff(
    gold: str, converted: str, test_case: str, generator: str
) -> List[Issue]:
    """Analyze differences and generate issues."""
    issues = []

    gold_lines = gold.strip().split("\n")
    converted_lines = converted.strip().split("\n")

    # Get diff
    diff = list(difflib.unified_diff(gold_lines, converted_lines, lineterm=""))

    issue_count = 0
    current_line = 0

    for i, line in enumerate(diff):
        if line.startswith("@@"):
            # Parse line number
            match = re.search(r"-(\d+)", line)
            if match:
                current_line = int(match.group(1))
        elif line.startswith("-") and not line.startswith("---"):
            expected = line[1:]
            # Find corresponding + line
            actual = ""
            for j in range(i + 1, min(i + 5, len(diff))):
                if diff[j].startswith("+") and not diff[j].startswith("+++"):
                    actual = diff[j][1:]
                    break

            if expected.strip():  # Only track non-empty differences
                category, severity, description = categorize_issue(expected, actual)
                issue_count += 1
                issues.append(
                    Issue(
                        id=f"{test_case}_{generator}_{issue_count:03d}",
                        category=category,
                        severity=severity,
                        test_case=test_case,
                        generator=generator,
                        description=description,
                        expected=expected[:100],
                        actual=actual[:100] if actual else "(missing)",
                        line_number=current_line,
                    )
                )
            current_line += 1

    return issues


def run_observation(loop_num: int) -> List[TestResult]:
    """OBSERVE phase: Convert all PDFs and compare with gold."""
    print(f"\n{'='*60}")
    print(f"🔍 OBSERVE - Loop {loop_num}")
    print(f"{'='*60}")

    results = []

    # Find all PDFs
    pdf_files = sorted(PDF_DIR.glob("*.pdf"))
    print(f"Found {len(pdf_files)} PDF files")

    for pdf_path in pdf_files:
        # Parse filename: testcase_generator.pdf
        parts = pdf_path.stem.rsplit("_", 1)
        if len(parts) != 2:
            continue

        test_case, generator = parts

        # Find gold file
        gold_path = GOLD_DIR / f"{test_case}.gold.md"
        if not gold_path.exists():
            continue

        print(f"\n  📄 {test_case} ({generator}):")

        # Convert
        converted_path = CONVERTED_DIR / f"{test_case}_{generator}.md"
        success = convert_pdf(pdf_path, converted_path)

        if not success:
            print(f"    ❌ Conversion failed")
            continue

        # Read files
        with open(gold_path) as f:
            gold_content = f.read()
        with open(converted_path) as f:
            converted_content = f.read()

        # Calculate similarity
        similarity = calculate_similarity(gold_content, converted_content)

        # Generate diff
        diff_summary = generate_diff(gold_content, converted_content)

        # Analyze issues
        issues = analyze_diff(gold_content, converted_content, test_case, generator)

        print(f"    Similarity: {similarity:.1f}%")
        print(f"    Issues: {len(issues)}")

        results.append(
            TestResult(
                test_case=test_case,
                generator=generator,
                pdf_path=str(pdf_path),
                gold_path=str(gold_path),
                converted_path=str(converted_path),
                similarity=similarity,
                issues=issues,
                diff_summary=diff_summary,
            )
        )

    return results


def run_orientation(results: List[TestResult], loop_num: int) -> Dict:
    """ORIENT phase: Analyze patterns and prioritize issues."""
    print(f"\n{'='*60}")
    print(f"🧭 ORIENT - Loop {loop_num}")
    print(f"{'='*60}")

    # Aggregate issues by category
    by_category = {}
    by_severity = {"critical": [], "major": [], "minor": []}
    by_test_case = {}
    by_generator = {}

    all_issues = []
    for result in results:
        for issue in result.issues:
            all_issues.append(issue)

            if issue.category not in by_category:
                by_category[issue.category] = []
            by_category[issue.category].append(issue)

            by_severity[issue.severity].append(issue)

            if issue.test_case not in by_test_case:
                by_test_case[issue.test_case] = []
            by_test_case[issue.test_case].append(issue)

            if issue.generator not in by_generator:
                by_generator[issue.generator] = []
            by_generator[issue.generator].append(issue)

    # Summary statistics
    avg_similarity = sum(r.similarity for r in results) / len(results) if results else 0
    perfect_matches = sum(1 for r in results if r.similarity >= 99)

    print(f"\n📊 Summary:")
    print(f"  Total tests: {len(results)}")
    print(f"  Average similarity: {avg_similarity:.1f}%")
    print(f"  Perfect matches: {perfect_matches}")
    print(f"  Total issues: {len(all_issues)}")

    print(f"\n📋 Issues by category:")
    for cat, issues in sorted(by_category.items(), key=lambda x: -len(x[1])):
        print(f"  {cat}: {len(issues)}")

    print(f"\n⚠️  Issues by severity:")
    for sev, issues in by_severity.items():
        print(f"  {sev}: {len(issues)}")

    print(f"\n🔧 Issues by generator:")
    for gen, issues in sorted(by_generator.items(), key=lambda x: -len(x[1])):
        print(f"  {gen}: {len(issues)}")

    return {
        "total_tests": len(results),
        "avg_similarity": avg_similarity,
        "perfect_matches": perfect_matches,
        "total_issues": len(all_issues),
        "by_category": {k: len(v) for k, v in by_category.items()},
        "by_severity": {k: len(v) for k, v in by_severity.items()},
        "by_generator": {k: len(v) for k, v in by_generator.items()},
        "by_test_case": {k: len(v) for k, v in by_test_case.items()},
    }


def run_decision(
    results: List[TestResult], orientation: Dict, loop_num: int
) -> List[Dict]:
    """DECIDE phase: Prioritize fixes based on impact."""
    print(f"\n{'='*60}")
    print(f"🎯 DECIDE - Loop {loop_num}")
    print(f"{'='*60}")

    # Group all issues
    all_issues = []
    for result in results:
        all_issues.extend(result.issues)

    # Find most impactful issues (appear in multiple tests)
    issue_patterns = {}
    for issue in all_issues:
        pattern = f"{issue.category}:{issue.description}"
        if pattern not in issue_patterns:
            issue_patterns[pattern] = {
                "count": 0,
                "examples": [],
                "severity": issue.severity,
            }
        issue_patterns[pattern]["count"] += 1
        if len(issue_patterns[pattern]["examples"]) < 3:
            issue_patterns[pattern]["examples"].append(
                {
                    "test": issue.test_case,
                    "expected": issue.expected,
                    "actual": issue.actual,
                }
            )

    # Sort by impact (count * severity weight)
    severity_weight = {"critical": 3, "major": 2, "minor": 1}
    ranked_patterns = sorted(
        issue_patterns.items(),
        key=lambda x: x[1]["count"] * severity_weight.get(x[1]["severity"], 1),
        reverse=True,
    )

    print(f"\n🔝 Top issues to fix:")
    fixes = []
    for i, (pattern, data) in enumerate(ranked_patterns[:10], 1):
        category, description = pattern.split(":", 1)
        print(f"\n  {i}. [{data['severity'].upper()}] {description}")
        print(f"     Category: {category}")
        print(f"     Occurrences: {data['count']}")

        fixes.append(
            {
                "rank": i,
                "pattern": pattern,
                "category": category,
                "description": description,
                "severity": data["severity"],
                "count": data["count"],
                "examples": data["examples"],
            }
        )

    return fixes


def save_loop_results(
    loop_num: int, results: List[TestResult], orientation: Dict, fixes: List[Dict]
):
    """Save loop results to files."""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    # Save observation results
    obs_file = OBSERVE_DIR / f"loop_{loop_num:03d}_{timestamp}.json"
    with open(obs_file, "w") as f:
        json.dump([asdict(r) for r in results], f, indent=2, default=str)

    # Save orientation analysis
    orient_file = ORIENT_DIR / f"loop_{loop_num:03d}_{timestamp}.json"
    with open(orient_file, "w") as f:
        json.dump(orientation, f, indent=2)

    # Save decision/fixes
    decide_file = DECIDE_DIR / f"loop_{loop_num:03d}_{timestamp}.json"
    with open(decide_file, "w") as f:
        json.dump(fixes, f, indent=2)

    # Create human-readable summary
    summary = f"""# OODA Loop {loop_num} Results
Generated: {datetime.now().isoformat()}

## Summary
- Total tests: {orientation['total_tests']}
- Average similarity: {orientation['avg_similarity']:.1f}%
- Perfect matches: {orientation['perfect_matches']}
- Total issues: {orientation['total_issues']}

## Issues by Category
"""
    for cat, count in sorted(orientation["by_category"].items(), key=lambda x: -x[1]):
        summary += f"- {cat}: {count}\n"

    summary += "\n## Top Fixes Needed\n"
    for fix in fixes[:10]:
        summary += f"\n### {fix['rank']}. {fix['description']}\n"
        summary += f"- Category: {fix['category']}\n"
        summary += f"- Severity: {fix['severity']}\n"
        summary += f"- Occurrences: {fix['count']}\n"
        summary += "- Examples:\n"
        for ex in fix["examples"][:2]:
            summary += f"  - Test: {ex['test']}\n"
            summary += f"    Expected: `{ex['expected'][:60]}...`\n"
            summary += f"    Actual: `{ex['actual'][:60]}...`\n"

    summary_file = ACT_DIR / f"loop_{loop_num:03d}_{timestamp}_summary.md"
    with open(summary_file, "w") as f:
        f.write(summary)

    print(f"\n📁 Results saved to:")
    print(f"  {obs_file}")
    print(f"  {orient_file}")
    print(f"  {decide_file}")
    print(f"  {summary_file}")


def main():
    print("=" * 60)
    print("OODA Loop Executor")
    print("=" * 60)

    # Get loop number from args
    loop_num = int(sys.argv[1]) if len(sys.argv) > 1 else 1

    # OBSERVE
    results = run_observation(loop_num)

    if not results:
        print("\n❌ No test results generated")
        sys.exit(1)

    # ORIENT
    orientation = run_orientation(results, loop_num)

    # DECIDE
    fixes = run_decision(results, orientation, loop_num)

    # Save results
    save_loop_results(loop_num, results, orientation, fixes)

    print(f"\n{'='*60}")
    print(f"✅ Loop {loop_num} complete")
    print(f"   Similarity: {orientation['avg_similarity']:.1f}%")
    print(f"   Perfect: {orientation['perfect_matches']}/{orientation['total_tests']}")
    print(f"   Issues: {orientation['total_issues']}")
    print(f"{'='*60}")

    # Return exit code based on perfection
    if (
        orientation["avg_similarity"] >= 95
        and orientation["perfect_matches"] >= len(results) * 0.8
    ):
        return 0  # Success
    return 1  # More work needed


if __name__ == "__main__":
    sys.exit(main())
