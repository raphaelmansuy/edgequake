#!/usr/bin/env python3
"""
Analyze orphaned features to determine which exist in code vs. documented only.
"""

import json
import subprocess
from pathlib import Path
from typing import Dict, List

# Load orphaned features
with open("/tmp/validation_69.json") as f:
    data = json.load(f)

orphaned = data["orphaned"]

print(f"Analyzing {len(orphaned)} orphaned features...\n")

# Keywords to search for each feature
FEATURE_KEYWORDS = {
    "FEAT0002": ["chunk", "TextChunk", "Chunker"],
    "FEAT0005": ["graph", "GraphBuilder", "construct_graph"],
    "FEAT0006": ["embed", "Embedder", "vector"],
    "FEAT0008": ["stream", "StreamingResponse", "SSE"],
    "FEAT0009": ["EntityExtractor", "normalize_entity"],
    "FEAT0010": ["Summarizer", "summarize", "summary"],
    "FEAT0011": ["Lineage", "lineage_tracker", "provenance"],
    "FEAT0012": ["progress", "ProgressReporter", "websocket"],
    "FEAT0013": ["CostTracker", "cost_tracking", "llm_cost"],
    "FEAT0014": ["LLMCache", "cache_response", "cached_llm"],
    "FEAT0015": ["tenant", "TenantIsolation", "workspace_id"],
    "FEAT0016": ["Workspace", "create_workspace", "workspace_manager"],
    "FEAT0017": ["Conversation", "conversation_id", "chat_history"],
    "FEAT0018": ["RateLimit", "rate_limiter", "throttle"],
    "FEAT0019": ["TaskQueue", "background_task", "async_job"],
    "FEAT0020": ["AuditLog", "audit_event", "event_log"],
    "FEAT0105": ["mix_search", "weighted_search", "hybrid_search"],
    "FEAT0106": ["bypass", "direct_llm", "no_rag"],
    "FEAT0107": ["keyword", "extract_keywords", "keyword_extraction"],
    "FEAT0108": ["truncate", "truncate_context", "context_window"],
    "FEAT0109": ["SotaQueryEngine", "sota_search", "rerank"],
    "FEAT0110": ["vector_filter", "filter_vectors", "threshold"],
    "FEAT0201": ["MemoryStorage", "in_memory", "HashMap"],
    "FEAT0304": ["gleaning", "GleaningExtractor", "iterative_extraction"],
    "FEAT0405": ["explore_graph", "graph_exploration", "expand_node"],
    "FEAT0406": ["task_status", "TaskStatus", "job_progress"],
    "FEAT0501": ["extract_text", "pdf_text", "PdfExtractor"],
    "FEAT0502": ["layout", "LayoutAnalyzer", "page_layout"],
    "FEAT0503": ["lattice", "table_detection", "LatticeEngine"],
    "FEAT0504": ["markdown", "render_markdown", "md_output"],
    "FEAT0505": ["heading", "HeaderDetection", "detect_headings"],
    "FEAT0801": ["api_key", "ApiKeyAuth", "x-api-key"],
    "FEAT0802": ["jwt", "JwtAuth", "Bearer"],
    "FEAT0803": ["rbac", "RoleBasedAccess", "permissions"],
    "FEAT1003": ["multi_column", "column_detection", "columns"],
    "FEAT1004": ["image_ocr", "ocr", "tesseract"],
    "FEAT1005": ["formula", "math", "equation"],
    "FEAT1006": ["llm_clean", "content_clean", "clean_with_llm"],
    "FEAT1022": ["structure_detection", "HeaderDetectionProcessor", "CaptionDetection"],
    "FEAT1023": ["image_convert", "ImageConverter", "png_to_webp"],
    "FEAT1024": ["llm_image", "vision_llm", "understand_image"],
    "FEAT1025": ["chart", "diagram", "extract_chart"],
}

exists = []
missing = []

for feat in orphaned:
    feat_id = feat["id"]
    feat_name = feat["name"]
    keywords = FEATURE_KEYWORDS.get(feat_id, [])

    if not keywords:
        missing.append((feat_id, feat_name, "No keywords"))
        continue

    # Search in Rust crates
    found = False
    for keyword in keywords:
        result = subprocess.run(
            ["grep", "-r", keyword, "edgequake/crates", "--include=*.rs"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            line_count = len(result.stdout.strip().split("\n"))
            if line_count > 0:
                exists.append((feat_id, feat_name, keyword, line_count))
                found = True
                break

    if not found:
        missing.append((feat_id, feat_name, "Not found in code"))

print("✅ Features that exist in code:")
for feat_id, feat_name, keyword, count in exists:
    print(f"  {feat_id} - {feat_name}")
    print(f"    Found '{keyword}' in {count} locations")

print(f"\n❌ Features not found in code ({len(missing)}):")
for feat_id, feat_name, reason in missing:
    print(f"  {feat_id} - {feat_name} ({reason})")

print(f"\n📊 Summary:")
print(f"  ✅ Exist in code: {len(exists)}/42 ({len(exists)/42*100:.1f}%)")
print(f"  ❌ Missing: {len(missing)}/42 ({len(missing)/42*100:.1f}%)")
