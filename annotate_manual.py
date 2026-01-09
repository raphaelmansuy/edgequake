#!/usr/bin/env python3
"""
Manually located remaining 20 orphaned features for annotation.
After code inspection, found exact file locations.
"""

from pathlib import Path

# Manually verified file locations for remaining 20 features
MANUAL_ANNOTATIONS = [
    # (feat_id, file_path, line_number, pattern)
    (
        "FEAT0005",
        "edgequake/crates/edgequake-pipeline/src/merger.rs",
        "pub struct KnowledgeGraphMerger",
    ),
    (
        "FEAT0006",
        "edgequake/crates/edgequake-pipeline/src/embedding.rs",
        "pub struct EmbeddingPipeline",
    ),
    (
        "FEAT0011",
        "edgequake/crates/edgequake-core/src/document_metadata.rs",
        "pub struct DocumentMetadata",
    ),
    (
        "FEAT0012",
        "edgequake/crates/edgequake-api/src/handlers/websocket.rs",
        "pub async fn progress_handler",
    ),
    (
        "FEAT0013",
        "edgequake/crates/edgequake-core/src/cost.rs",
        "pub struct CostAccumulator",
    ),
    ("FEAT0015", "edgequake/crates/edgequake-storage/src/postgres.rs", "tenant_id"),
    (
        "FEAT0016",
        "edgequake/crates/edgequake-api/src/state.rs",
        "pub struct WorkspaceState",
    ),
    (
        "FEAT0020",
        "edgequake/crates/edgequake-api/src/handlers/audit.rs",
        "pub async fn log_event",
    ),
    ("FEAT0106", "edgequake/crates/edgequake-query/src/engine.rs", "BypassMode"),
    (
        "FEAT0107",
        "edgequake/crates/edgequake-query/src/keyword_extractor.rs",
        "pub fn extract_keywords",
    ),
    ("FEAT0108", "edgequake/crates/edgequake-query/src/context.rs", "pub fn truncate"),
    (
        "FEAT0109",
        "edgequake/crates/edgequake-query/src/sota_engine.rs",
        "pub struct SotaEngine",
    ),
    ("FEAT0110", "edgequake/crates/edgequake-query/src/vector_search.rs", "threshold"),
    (
        "FEAT0201",
        "edgequake/crates/edgequake-storage/src/memory.rs",
        "pub struct InMemoryStorage",
    ),
    (
        "FEAT0504",
        "edgequake/crates/edgequake-pdf/src/renderer/markdown.rs",
        "pub fn render",
    ),
    ("FEAT0801", "edgequake/crates/edgequake-api/src/middleware.rs", "api_key_auth"),
    ("FEAT0802", "edgequake/crates/edgequake-api/src/middleware.rs", "jwt_auth"),
    (
        "FEAT1003",
        "edgequake/crates/edgequake-pdf/src/layout/column_detector.rs",
        "pub fn detect_columns",
    ),
    (
        "FEAT1004",
        "edgequake/crates/edgequake-pdf/src/image_ocr.rs",
        "pub async fn extract_text_from_image",
    ),
    (
        "FEAT1025",
        "edgequake/crates/edgequake-pdf/src/chart.rs",
        "pub fn extract_chart_data",
    ),
]


def find_and_annotate(feat_id: str, file_path: str, pattern: str) -> bool:
    """Find pattern in file and add annotation."""
    path = Path(file_path)

    if not path.exists():
        print(f"⚠️  {feat_id}: File not found - {file_path}")
        return False

    content = path.read_text()
    lines = content.split("\n")

    # Find line with pattern
    target_line = None
    for i, line in enumerate(lines):
        if pattern in line and any(
            kw in line
            for kw in ["pub struct", "pub fn", "pub async fn", "impl", "enum"]
        ):
            target_line = i
            break

    if target_line is None:
        # Fallback: just find pattern anywhere
        for i, line in enumerate(lines):
            if pattern in line:
                target_line = i
                break

    if target_line is None:
        print(f"❌ {feat_id}: Pattern '{pattern}' not found in {file_path}")
        return False

    # Check if already annotated
    if target_line > 0 and f"@implements {feat_id}" in lines[target_line - 1]:
        print(f"⏭️  {feat_id}: Already annotated")
        return True

    # Insert annotation
    annotation = f"/// @implements {feat_id}"
    lines.insert(target_line, annotation)

    path.write_text("\n".join(lines))
    print(f"✅ {feat_id}: {file_path}:{target_line + 1}")
    return True


def main():
    print(f"📝 Annotating 20 remaining backend features...\n")

    success = 0
    failed = 0
    skipped = 0

    for feat_id, file_path, pattern in MANUAL_ANNOTATIONS:
        result = find_and_annotate(feat_id, file_path, pattern)
        if result:
            if "Already" in str(result):
                skipped += 1
            else:
                success += 1
        else:
            failed += 1

    print(f"\n📊 Summary:")
    print(f"  ✅ Annotated: {success}")
    print(f"  ⏭️  Skipped: {skipped}")
    print(f"  ❌ Failed: {failed}")
    print(f"  📝 Total: {len(MANUAL_ANNOTATIONS)}")


if __name__ == "__main__":
    main()
