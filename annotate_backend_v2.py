#!/usr/bin/env python3
"""
Find primary file location for each orphaned feature and add @implements annotation.
"""

import re
import subprocess
from pathlib import Path
from typing import Optional, Tuple

# Features that exist and need annotations
FEATURES_TO_ANNOTATE = [
    ("FEAT0002", "TextChunk", "edgequake/crates/edgequake-pipeline/src"),
    ("FEAT0005", "graph_builder", "edgequake/crates/edgequake-core/src"),
    ("FEAT0006", "Embedder", "edgequake/crates/edgequake-pipeline/src"),
    ("FEAT0008", "streaming", "edgequake/crates/edgequake-api/src"),
    (
        "FEAT0009",
        "EntityExtractor",
        "edgequake/crates/edgequake-pipeline/src/extractor.rs",
    ),
    ("FEAT0010", "Summarizer", "edgequake/crates/edgequake-pipeline/src/summarizer.rs"),
    ("FEAT0011", "LineageTracker", "edgequake/crates/edgequake-core/src"),
    ("FEAT0012", "progress_ws", "edgequake/crates/edgequake-api/src"),
    ("FEAT0013", "CostTracker", "edgequake/crates/edgequake-core/src"),
    ("FEAT0014", "LLMCache", "edgequake/crates/edgequake-llm/src/cache.rs"),
    ("FEAT0015", "TenantStorage", "edgequake/crates/edgequake-storage/src"),
    ("FEAT0016", "WorkspaceState", "edgequake/crates/edgequake-api/src/state.rs"),
    ("FEAT0017", "Conversation", "edgequake/crates/edgequake-api/src"),
    ("FEAT0018", "RateLimiter", "edgequake/crates/edgequake-api/src"),
    ("FEAT0019", "TaskQueue", "edgequake/crates/edgequake-api/src"),
    ("FEAT0020", "AuditLog", "edgequake/crates/edgequake-api/src"),
    ("FEAT0106", "bypass", "edgequake/crates/edgequake-query/src"),
    ("FEAT0107", "extract_keywords", "edgequake/crates/edgequake-query/src"),
    ("FEAT0108", "truncate_context", "edgequake/crates/edgequake-query/src"),
    (
        "FEAT0109",
        "SotaQueryEngine",
        "edgequake/crates/edgequake-query/src/sota_engine.rs",
    ),
    ("FEAT0110", "VectorFilter", "edgequake/crates/edgequake-query/src"),
    ("FEAT0201", "MemoryStorage", "edgequake/crates/edgequake-storage/src"),
    ("FEAT0304", "GleaningExtractor", "edgequake/crates/edgequake-pipeline/src"),
    ("FEAT0406", "task_status", "edgequake/crates/edgequake-api/src"),
    ("FEAT0501", "PdfExtractor", "edgequake/crates/edgequake-pdf/src/extractor.rs"),
    ("FEAT0502", "LayoutAnalyzer", "edgequake/crates/edgequake-pdf/src/layout"),
    (
        "FEAT0503",
        "LatticeEngine",
        "edgequake/crates/edgequake-pdf/src/backend/lattice.rs",
    ),
    ("FEAT0504", "render_to_markdown", "edgequake/crates/edgequake-pdf/src"),
    (
        "FEAT0505",
        "HeaderDetectionProcessor",
        "edgequake/crates/edgequake-pdf/src/processors",
    ),
    ("FEAT0801", "ApiKeyLayer", "edgequake/crates/edgequake-api/src"),
    ("FEAT0802", "JwtAuth", "edgequake/crates/edgequake-api/src"),
    ("FEAT0803", "permissions", "edgequake/crates/edgequake-api/src"),
    ("FEAT1003", "detect_multi_column", "edgequake/crates/edgequake-pdf/src"),
    ("FEAT1004", "image_to_text", "edgequake/crates/edgequake-pdf/src"),
    ("FEAT1005", "detect_formula", "edgequake/crates/edgequake-pdf/src"),
    (
        "FEAT1022",
        "HeaderDetectionProcessor",
        "edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs",
    ),
    ("FEAT1024", "Vision", "edgequake/crates/edgequake-pdf/src"),
    ("FEAT1025", "chart_to_data", "edgequake/crates/edgequake-pdf/src"),
]


def find_file_with_pattern(pattern: str, search_dir: str) -> Optional[Tuple[str, int]]:
    """Find file containing pattern and return (file_path, line_number)."""
    # Check if search_dir is a specific file
    if search_dir.endswith(".rs"):
        path = Path(search_dir)
        if path.exists():
            content = path.read_text()
            lines = content.split("\n")
            for i, line in enumerate(lines, 1):
                if pattern in line and (
                    "struct" in line
                    or "pub fn" in line
                    or "trait" in line
                    or "impl" in line
                ):
                    return (str(path), i)
        return None

    # Search directory
    result = subprocess.run(
        ["grep", "-rn", pattern, search_dir, "--include=*.rs"],
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        return None

    lines = result.stdout.strip().split("\n")
    for line in lines:
        if "test" in line.lower():
            continue  # Skip test files

        match = re.match(r"([^:]+):(\d+):(.*)", line)
        if match:
            file_path, line_num, code = match.groups()
            # Prefer struct/fn/trait definitions
            if any(kw in code for kw in ["struct", "pub fn", "trait", "impl"]):
                return (file_path, int(line_num))

    # Fallback to first match
    if lines:
        match = re.match(r"([^:]+):(\d+):", lines[0])
        if match:
            return (match.group(1), int(match.group(2)))

    return None


def add_annotation(file_path: str, line_num: int, feat_id: str) -> bool:
    """Add @implements annotation above the specified line."""
    path = Path(file_path)
    content = path.read_text()
    lines = content.split("\n")

    # Check if already annotated
    if line_num > 1 and f"@implements {feat_id}" in lines[line_num - 2]:
        return False  # Already annotated

    # Insert annotation
    annotation = f"/// @implements {feat_id}"
    lines.insert(line_num - 1, annotation)

    path.write_text("\n".join(lines))
    return True


def annotate_all():
    """Find and annotate all features."""
    print(f"📝 Annotating {len(FEATURES_TO_ANNOTATE)} backend features...\n")

    success = 0
    skip = 0
    failed = 0

    for feat_id, pattern, search_dir in FEATURES_TO_ANNOTATE:
        location = find_file_with_pattern(pattern, search_dir)

        if not location:
            print(f"❌ {feat_id}: Pattern '{pattern}' not found in {search_dir}")
            failed += 1
            continue

        file_path, line_num = location

        if add_annotation(file_path, line_num, feat_id):
            print(f"✅ {feat_id}: {file_path}:{line_num}")
            success += 1
        else:
            print(f"⏭️  {feat_id}: Already annotated ({file_path})")
            skip += 1

    print(f"\n📊 Summary:")
    print(f"  ✅ Annotated: {success}")
    print(f"  ⏭️  Skipped: {skip}")
    print(f"  ❌ Failed: {failed}")
    print(f"  📝 Total: {len(FEATURES_TO_ANNOTATE)}")


if __name__ == "__main__":
    annotate_all()
