#!/usr/bin/env python3
"""Measure live query modes; keep retrieved content and answers out of reports.

Answer generation is opt-in and restricted to the explicitly selected document.
Workspace-wide cases use context_only and disable reranking to keep context local.
Use a new question set for cold application-cache measurements; repeated runs
measure warm application caches. Database-only plans are measured separately.
"""

import argparse
import json
import math
import statistics
import time
import urllib.error
import urllib.request
from pathlib import Path

MODES = ("naive", "local", "global", "hybrid", "mix", "bypass")
QUESTIONS = (
    "Summarize CoinRAG's main claims about information nuggets and KV cache reuse.",
    "What inefficiency does CoinRAG identify in coarse-grained retrieved chunks?",
    "How does CoinRAG reuse KV caches using contextualized information nuggets?",
    "How does CoinRAG address information redundancy while preserving answer quality?",
    "What experimental evidence and limitations does the CoinRAG paper report?",
)


def percentile(samples, fraction):
    ordered = sorted(samples)
    return ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]


def read_stream(response, start):
    events, first_token, answer, context, done = [], None, "", {}, {}
    for raw in response:
        line = raw.decode("utf-8").strip()
        if not line.startswith("data:"):
            continue
        event = json.loads(line[5:].strip())
        kind = event.get("type")
        events.append(kind)
        if kind == "context":
            context = event
        elif kind == "token":
            if first_token is None:
                first_token = (time.perf_counter() - start) * 1000
            answer += event.get("content", "")
        elif kind == "done":
            done = event
    payload_events = [kind for kind in events if kind != "thinking"]
    ordered = (bool(payload_events) and payload_events[0] == "context" and events[-1] == "done"
               and events.count("context") == 1 and events.count("done") == 1
               and "token" in events and "error" not in events)
    return {
        "answer": done.get("answer") or answer,
        "mode": context.get("query_mode"), "sources": context.get("sources", []),
        "stats": done.get("stats", {}), "stream_order_ok": ordered,
        "first_token_ms": round(first_token, 2) if first_token is not None else None,
        "token_events": events.count("token"), "event_types": list(dict.fromkeys(events)),
    }


def run_case(args, mode, scope, sample):
    generating = args.generate and scope == "document"
    body = {
        "query": args.question_prefix.replace("{mode}", mode) + QUESTIONS[sample % len(QUESTIONS)],
        "mode": mode,
        "context_only": not generating,
        "include_references": True,
        "include_subgraph": True,
        "enable_rerank": False,
        "max_results": 10,
        "response_type": "Five concise bullet points",
    }
    if args.stream:
        # These REST-only fields are not part of StreamQueryRequest.
        for field in ("context_only", "include_references", "enable_rerank", "max_results"):
            body.pop(field)
        body["stream_format"] = "v2"
    if scope == "document":
        body["document_filter"] = {"document_ids": [args.document]}
    if args.preset_keywords:
        body["hl_keywords"] = ["retrieval augmented generation", "KV cache reuse"]
        body["ll_keywords"] = ["CoinRAG", "information nuggets"]
    request = urllib.request.Request(
        args.base_url.rstrip("/") + ("/api/v1/query/stream" if args.stream else "/api/v1/query"),
        data=json.dumps(body).encode(),
        headers={
            "Content-Type": "application/json",
            "X-Tenant-ID": args.tenant,
            "X-Workspace-ID": args.workspace,
        },
        method="POST",
    )
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(request, timeout=args.timeout) as response:
            status = response.status
            data = read_stream(response, start) if args.stream else json.load(response)
    except urllib.error.HTTPError as error:
        status, data = error.code, json.load(error)
    except (OSError, ValueError) as error:
        return {"mode": mode, "scope": scope, "sample": sample, "ok": False,
                "error_type": type(error).__name__}
    elapsed = (time.perf_counter() - start) * 1000
    sources = data.get("sources", [])
    documents = {source["document_id"] for source in sources if source.get("document_id")}
    scope_ok = scope != "document" or (
        documents <= {args.document}
        and all(source.get("document_id") == args.document
                for source in sources if source.get("source_type") == "chunk")
    )
    mode_ok = data.get("mode") == mode
    coverage_ok = (not sources) if mode == "bypass" else bool(sources)
    stream_ok = data.get("stream_order_ok", not args.stream)
    if args.require_incremental:
        stream_ok = stream_ok and data.get("token_events", 0) > 1
    answer = data.get("answer", "")
    content_ok = len(answer.strip()) > 100 if generating else not answer
    return {
        "mode": mode, "scope": scope, "sample": sample,
        "ok": status == 200 and scope_ok and content_ok and mode_ok and coverage_ok and stream_ok,
        "mode_ok": mode_ok, "source_coverage_ok": coverage_ok,
        "stream_order_ok": stream_ok, "first_token_ms": data.get("first_token_ms"),
        "token_events": data.get("token_events"), "event_types": data.get("event_types"),
        "http_status": status, "elapsed_ms": round(elapsed, 2),
        "document_scope_ok": scope_ok, "answer_chars": len(answer),
        "source_count": len(sources), "stats": data.get("stats", {}),
        "error_code": data.get("code"),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="http://localhost:8091")
    parser.add_argument("--tenant", required=True)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--document", required=True)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--generate", action="store_true",
                        help="Send selected-document context to its configured LLM")
    parser.add_argument("--preset-keywords", action="store_true")
    parser.add_argument("--stream", action="store_true")
    parser.add_argument("--question-prefix", default="",
                        help="Optional prefix; {mode} makes prompts distinct across modes")
    parser.add_argument("--require-incremental", action="store_true",
                        help="Require multiple SSE token events (use fresh prompts)")
    parser.add_argument("--scopes", nargs="+", choices=("document", "workspace"),
                        default=["document", "workspace"])
    parser.add_argument("--modes", nargs="+", choices=MODES, default=MODES)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.samples < 1 or args.timeout <= 0:
        parser.error("samples and timeout must be positive")
    if args.require_incremental and not args.stream:
        parser.error("--require-incremental requires --stream")
    if args.stream and (not args.generate or args.scopes != ["document"]):
        parser.error("streaming always generates: require --generate --scopes document")
    report = {"generate_document_only": args.generate, "streaming": args.stream,
              "preset_keywords": args.preset_keywords, "require_incremental": args.require_incremental,
              "cache_policy": "Server configuration; distinct questions per sample up to five",
              "rows": [], "summary": []}
    for scope in args.scopes:
        for mode in args.modes:
            rows = [run_case(args, mode, scope, sample) for sample in range(args.samples)]
            report["rows"].extend(rows)
            elapsed = [row["elapsed_ms"] for row in rows if row["ok"]]
            summary = {"mode": mode, "scope": scope, "samples": len(rows),
                       "passed": sum(row["ok"] for row in rows)}
            if elapsed:
                summary.update(p50_ms=round(statistics.median(elapsed), 2),
                               p95_ms=round(percentile(elapsed, .95), 2),
                               max_ms=round(max(elapsed), 2))
            report["summary"].append(summary)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            print(json.dumps(summary), flush=True)
    return 0 if all(row["ok"] for row in report["rows"]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
