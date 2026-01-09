#!/usr/bin/env python3
"""
Add @implements FEATXXXX annotations to Rust backend code based on features.md.
Maps documented features to their implementation files.
"""

import re
from pathlib import Path
from typing import Dict, List, Tuple

# Mapping of feature IDs to their Rust implementation files
FEATURE_MAPPINGS = {
    # Core RAG (FEAT00XX)
    "FEAT0002": [
        (
            "edgequake/crates/edgequake-pipeline/src/chunker.rs",
            1,
            "pub struct TextChunker",
        ),
    ],
    "FEAT0005": [
        (
            "edgequake/crates/edgequake-core/src/graph_builder.rs",
            1,
            "pub struct GraphBuilder",
        ),
    ],
    "FEAT0006": [
        (
            "edgequake/crates/edgequake-pipeline/src/embedder.rs",
            1,
            "pub struct Embedder",
        ),
    ],
    "FEAT0008": [
        ("edgequake/crates/edgequake-api/src/handlers/query.rs", 1, "async fn query"),
    ],
    "FEAT0009": [
        (
            "edgequake/crates/edgequake-pipeline/src/extractor.rs",
            1,
            "pub trait EntityExtractor",
        ),
    ],
    "FEAT0010": [
        (
            "edgequake/crates/edgequake-pipeline/src/summarizer.rs",
            1,
            "pub struct Summarizer",
        ),
    ],
    "FEAT0011": [
        (
            "edgequake/crates/edgequake-core/src/lineage.rs",
            1,
            "pub struct LineageTracker",
        ),
    ],
    "FEAT0012": [
        (
            "edgequake/crates/edgequake-api/src/handlers/progress.rs",
            1,
            "pub fn progress_ws",
        ),
    ],
    "FEAT0013": [
        (
            "edgequake/crates/edgequake-core/src/cost_tracker.rs",
            1,
            "pub struct CostTracker",
        ),
    ],
    "FEAT0014": [
        ("edgequake/crates/edgequake-llm/src/cache.rs", 1, "pub struct LLMCache"),
    ],
    "FEAT0015": [
        (
            "edgequake/crates/edgequake-storage/src/tenant.rs",
            1,
            "pub trait TenantIsolation",
        ),
    ],
    "FEAT0016": [
        (
            "edgequake/crates/edgequake-api/src/handlers/workspace.rs",
            1,
            "pub async fn create_workspace",
        ),
    ],
    "FEAT0017": [
        (
            "edgequake/crates/edgequake-api/src/handlers/conversation.rs",
            1,
            "pub async fn create_conversation",
        ),
    ],
    "FEAT0018": [
        (
            "edgequake/crates/edgequake-api/src/middleware/rate_limit.rs",
            1,
            "pub struct RateLimitLayer",
        ),
    ],
    "FEAT0019": [
        ("edgequake/crates/edgequake-api/src/tasks.rs", 1, "pub struct TaskQueue"),
    ],
    "FEAT0020": [
        ("edgequake/crates/edgequake-api/src/audit.rs", 1, "pub struct AuditLog"),
    ],
    # Query Engine (FEAT01XX)
    "FEAT0105": [
        (
            "edgequake/crates/edgequake-query/src/mix_search.rs",
            1,
            "pub async fn mix_search",
        ),
    ],
    "FEAT0106": [
        (
            "edgequake/crates/edgequake-query/src/bypass.rs",
            1,
            "pub async fn bypass_mode",
        ),
    ],
    "FEAT0107": [
        (
            "edgequake/crates/edgequake-query/src/keyword.rs",
            1,
            "pub fn extract_keywords",
        ),
    ],
    "FEAT0108": [
        (
            "edgequake/crates/edgequake-query/src/truncate.rs",
            1,
            "pub fn truncate_context",
        ),
    ],
    "FEAT0109": [
        (
            "edgequake/crates/edgequake-query/src/sota_engine.rs",
            1,
            "pub struct SotaQueryEngine",
        ),
    ],
    "FEAT0110": [
        (
            "edgequake/crates/edgequake-query/src/vector_filter.rs",
            1,
            "pub fn filter_vectors",
        ),
    ],
    # Storage (FEAT02XX)
    "FEAT0201": [
        (
            "edgequake/crates/edgequake-storage/src/memory.rs",
            1,
            "pub struct MemoryStorage",
        ),
    ],
    # Pipeline (FEAT03XX)
    "FEAT0304": [
        (
            "edgequake/crates/edgequake-pipeline/src/gleaning.rs",
            1,
            "pub struct GleaningExtractor",
        ),
    ],
    # API (FEAT04XX)
    "FEAT0405": [
        (
            "edgequake/crates/edgequake-api/src/handlers/graph.rs",
            1,
            "pub async fn explore_graph",
        ),
    ],
    "FEAT0406": [
        (
            "edgequake/crates/edgequake-api/src/handlers/tasks.rs",
            1,
            "pub async fn task_status",
        ),
    ],
    # PDF (FEAT05XX)
    "FEAT0501": [
        ("edgequake/crates/edgequake-pdf/src/extractor.rs", 1, "pub fn extract_text"),
    ],
    "FEAT0502": [
        (
            "edgequake/crates/edgequake-pdf/src/layout/analyzer.rs",
            1,
            "pub struct LayoutAnalyzer",
        ),
    ],
    "FEAT0503": [
        (
            "edgequake/crates/edgequake-pdf/src/backend/lattice.rs",
            1,
            "pub struct LatticeEngine",
        ),
    ],
    "FEAT0504": [
        ("edgequake/crates/edgequake-pdf/src/renderer.rs", 1, "pub fn render_markdown"),
    ],
    "FEAT0505": [
        (
            "edgequake/crates/edgequake-pdf/src/processors/processor.rs",
            1,
            "pub struct HeaderDetectionProcessor",
        ),
    ],
    # Auth (FEAT08XX)
    "FEAT0801": [
        (
            "edgequake/crates/edgequake-api/src/middleware/auth.rs",
            1,
            "pub struct ApiKeyAuth",
        ),
    ],
    "FEAT0802": [
        (
            "edgequake/crates/edgequake-api/src/middleware/jwt.rs",
            1,
            "pub struct JwtAuth",
        ),
    ],
    "FEAT0803": [
        (
            "edgequake/crates/edgequake-api/src/middleware/rbac.rs",
            1,
            "pub struct RbacMiddleware",
        ),
    ],
    # Advanced PDF (FEAT10XX)
    "FEAT1003": [
        (
            "edgequake/crates/edgequake-pdf/src/layout/multi_column.rs",
            1,
            "pub fn detect_columns",
        ),
    ],
    "FEAT1004": [
        (
            "edgequake/crates/edgequake-pdf/src/image_ocr.rs",
            1,
            "pub async fn extract_image_text",
        ),
    ],
    "FEAT1005": [
        (
            "edgequake/crates/edgequake-pdf/src/formula_detector.rs",
            1,
            "pub fn detect_formulas",
        ),
    ],
    "FEAT1006": [
        (
            "edgequake/crates/edgequake-pdf/src/llm_cleaner.rs",
            1,
            "pub async fn clean_with_llm",
        ),
    ],
    "FEAT1022": [
        (
            "edgequake/crates/edgequake-pdf/src/processors/structure_detection.rs",
            1,
            "pub struct HeaderDetectionProcessor",
        ),
    ],
    "FEAT1023": [
        (
            "edgequake/crates/edgequake-pdf/src/image_converter.rs",
            1,
            "pub fn convert_image_format",
        ),
    ],
    "FEAT1024": [
        (
            "edgequake/crates/edgequake-pdf/src/llm_image.rs",
            1,
            "pub async fn understand_image",
        ),
    ],
    "FEAT1025": [
        (
            "edgequake/crates/edgequake-pdf/src/chart_extractor.rs",
            1,
            "pub async fn extract_chart_data",
        ),
    ],
}


def add_annotation_to_file(
    file_path: str, line_num: int, feat_id: str, pattern: str
) -> bool:
    """Add @implements annotation before the specified pattern."""
    path = Path(file_path)

    if not path.exists():
        print(f"❌ File not found: {file_path}")
        return False

    content = path.read_text()
    lines = content.split("\n")

    # Find the line matching the pattern
    target_line = None
    for i, line in enumerate(lines):
        if pattern in line:
            target_line = i
            break

    if target_line is None:
        print(f"⚠️  Pattern not found in {file_path}: {pattern}")
        return False

    # Check if annotation already exists
    if target_line > 0 and f"@implements {feat_id}" in lines[target_line - 1]:
        print(f"⏭️  Already annotated: {file_path} {feat_id}")
        return True

    # Insert annotation as a doc comment
    annotation = f"/// @implements {feat_id}"
    lines.insert(target_line, annotation)

    # Write back
    path.write_text("\n".join(lines))
    print(f"✅ Added {feat_id} to {file_path}")
    return True


def annotate_backend():
    """Add annotations to all backend files."""
    total = len(FEATURE_MAPPINGS)
    success = 0
    failed = 0
    skipped = 0

    print(f"📝 Adding @implements annotations to {total} backend features...\n")

    for feat_id, locations in FEATURE_MAPPINGS.items():
        for file_path, line_num, pattern in locations:
            result = add_annotation_to_file(file_path, line_num, feat_id, pattern)
            if result:
                if "Already" in str(result):
                    skipped += 1
                else:
                    success += 1
            else:
                failed += 1

    print(f"\n📊 Summary:")
    print(f"  ✅ Added: {success}")
    print(f"  ⏭️  Skipped (already annotated): {skipped}")
    print(f"  ❌ Failed: {failed}")
    print(f"  📝 Total: {total}")


if __name__ == "__main__":
    annotate_backend()
