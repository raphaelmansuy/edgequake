#!/usr/bin/env python3
"""SPEC-182 demo driver: build a Normal RAG workspace that embeds text and images, load the
curated corpus, then run the searches the article's screenshots show.

Runs against a live backend. Every call is real: real Vertex `gemini-embedding-2` embeddings,
real Postgres storage, real HTTP. Nothing here stubs a vector.

    python3 run_demo.py --base http://127.0.0.1:8099

Writes `api/transcript.json` (the exact request/response of each step, for the article's API
section) and `api/verdict.json` (one row per expected hit, PASS/FAIL).
"""
from __future__ import annotations

import argparse
import json
import mimetypes
import sys
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

HERE = Path(__file__).resolve().parent
IMAGES = HERE.parent / "images"
USER = "00000000-0000-0000-0000-000000000001"


# --------------------------------------------------------------------------- HTTP


def call(base, method, path, *, body=None, headers=None, raw=None, ctype=None, timeout=300):
    data = raw
    if data is None and body is not None:
        data = json.dumps(body).encode()
        ctype = "application/json"
    req = urllib.request.Request(base + path, data=data, method=method)
    if ctype:
        req.add_header("Content-Type", ctype)
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            payload = r.read()
            try:
                return r.status, json.loads(payload or b"null")
            except ValueError:
                return r.status, {"raw": payload[:400].decode(errors="replace")}
    except urllib.error.HTTPError as e:
        text = e.read().decode(errors="replace")
        try:
            return e.code, json.loads(text)
        except ValueError:
            return e.code, {"raw": text[:400]}


def multipart(fields, files):
    """`fields` are (name, text); `files` are (name, filename, bytes)."""
    boundary = "----eq182" + uuid.uuid4().hex
    out = b""
    for name, value in fields:
        out += f"--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n".encode()
        out += value.encode() + b"\r\n"
    for name, filename, blob in files:
        mime = mimetypes.guess_type(filename)[0] or "application/octet-stream"
        out += (
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"; "
            f"filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        ).encode()
        out += blob + b"\r\n"
    out += f"--{boundary}--\r\n".encode()
    return out, f"multipart/form-data; boundary={boundary}"


# --------------------------------------------------------------------------- transcript


class Transcript:
    """Records each step in the shape the article quotes: request, status, response excerpt."""

    def __init__(self):
        self.steps = []

    def add(self, name, request, status, response, ms=None, note=None):
        row = {
            "step": name,
            "request": request,
            "status": status,
            "response": response,
        }
        if ms is not None:
            row["duration_ms"] = ms
        if note:
            row["note"] = note
        self.steps.append(row)
        print(f"  {name}: {status}")
        return row

    def dump(self, path):
        path.write_text(json.dumps({"steps": self.steps}, indent=2, ensure_ascii=False))


# --------------------------------------------------------------------------- demo


def chunk_docs(response):
    """Document ids of the chunk sources, best first. Entities are absent on a normal ws."""
    sources = response.get("sources") or []
    return [s["document_id"] for s in sources if s.get("source_type") == "chunk"]


def top_scores(response, n=5):
    sources = [s for s in (response.get("sources") or []) if s.get("source_type") == "chunk"]
    return [
        {
            "document_id": s.get("document_id"),
            "score": round(float(s.get("score") or 0.0), 5),
            "snippet": (s.get("snippet") or "")[:120],
        }
        for s in sources[:n]
    ]


def wait_ready(base, doc_id, headers, deadline_s=300):
    """Poll until the ingest reaches a terminal state.

    WHY `indexed` is terminal for a normal workspace: it runs no extraction and no graph
    write, so there is nothing after the vectors land. `completed` appears too, depending on
    whether the row reports the pipeline stage or the retrieval stage, so both are accepted.
    """
    terminal = {"completed", "indexed", "failed", "partial_failure", "cancelled"}
    deadline = time.time() + deadline_s
    last = "unknown"
    while time.time() < deadline:
        st, doc = call(base, "GET", f"/api/v1/documents/{doc_id}", headers=headers)
        last = (doc or {}).get("status", "unknown")
        if last in terminal:
            return last, doc
        time.sleep(1.5)
    return last, {}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:8090")
    ap.add_argument("--reuse", default=None, help="tenant_id/workspace_id to reuse")
    args = ap.parse_args()
    base = args.base.rstrip("/")
    t0 = time.time()
    tr = Transcript()

    corpus = json.loads((IMAGES / "captions.json").read_text())
    names = {img["file"]: img["caption"] for img in corpus["images"]}

    print("\n== 1. create scope")
    suffix = uuid.uuid4().hex[:6]
    st, tenant = call(base, "POST", "/api/v1/tenants", body={"name": f"SPEC-182 corpus {suffix}"})
    tenant_id = tenant.get("id")
    tr.add(
        "create_tenant",
        {"POST": "/api/v1/tenants", "body": {"name": f"SPEC-182 corpus {suffix}"}},
        st,
        {"id": tenant_id, "name": tenant.get("name")},
    )

    body = {
        "name": f"Picture library {suffix}",
        "rag_profile": "normal",
        "embedding_modalities": "text,image",
        "embedding_provider": "vertexai",
        "embedding_model": "gemini-embedding-2",
        "embedding_dimension": 1536,
        "llm_provider": "ollama",
        "llm_model": "gemma4:latest",
    }
    st, ws = call(
        base,
        "POST",
        f"/api/v1/tenants/{tenant_id}/workspaces",
        body=body,
        headers={"X-User-ID": USER},
    )
    if st != 201:
        print(json.dumps(ws, indent=2)[:1200])
        return 1
    ws_id = ws["id"]
    caps = ws.get("rag_profile_capabilities", {})
    tr.add(
        "create_workspace",
        {"POST": f"/api/v1/tenants/{tenant_id}/workspaces", "body": body},
        st,
        {
            "id": ws_id,
            "rag_profile": ws.get("rag_profile"),
            "embedding_modalities": ws.get("embedding_modalities"),
            "embedding_model": ws.get("embedding_model"),
            "embedding_dimension": ws.get("embedding_dimension"),
            "rag_profile_capabilities": caps,
        },
        note="embeds_images=true, extracts_graph=false is the whole contract",
    )
    h = {"X-Tenant-ID": tenant_id, "X-User-ID": USER, "X-Workspace-ID": ws_id}

    print("\n== 2. ingest 20 pictures")
    docs = {}
    for img in corpus["images"]:
        path = IMAGES / img["file"]
        blob = path.read_bytes()
        # WHY the caption is the upload filename: the file-upload route derives the document
        # title from the filename and takes no separate title field, so this is the only way
        # the documents list shows "A beagle with its tongue out" instead of
        # "20-beagle-portrait.jpg". The on-disk corpus keeps short, sortable names.
        upload_name = f"{img['caption']}.jpg"
        raw, ctype = multipart([], [("file", upload_name, blob)])
        st, doc = call(
            base, "POST", "/api/v1/documents/upload", raw=raw, ctype=ctype, headers=h
        )
        doc_id = doc.get("document_id") or doc.get("id")
        docs[img["file"]] = doc_id
        tr.add(
            f"upload:{img['file']}",
            {
                "POST": "/api/v1/documents/upload",
                "multipart": {"file": upload_name},
                "bytes": len(blob),
            },
            st,
            {"document_id": doc_id, "status": doc.get("status")},
        )
        if st not in (200, 201, 202):
            print(json.dumps(doc)[:500])

    print("\n== 3. wait for ingestion")
    statuses = {}
    for name, doc_id in docs.items():
        if not doc_id:
            statuses[name] = "no-id"
            continue
        status, _ = wait_ready(base, doc_id, h)
        statuses[name] = status
    bad = {k: v for k, v in statuses.items() if v not in ("completed", "indexed")}
    tr.add(
        "ingest_complete",
        {"GET": "/api/v1/documents/{id} until status is terminal"},
        "ok" if not bad else "incomplete",
        {"statuses": statuses, "not_ready": bad},
        note="a normal workspace runs no extraction, so ready means chunks + vectors exist",
    )

    print("\n== 4. text -> image")
    verdicts = []
    for img in corpus["images"]:
        for case in img["expect"]:
            before = time.time()
            st, res = call(
                base,
                "POST",
                "/api/v1/query",
                body={
                    "query": case["query"],
                    "context_only": True,
                    "document_filter": {"doc_kinds": ["image"]},
                },
                headers=h,
            )
            ms = int((time.time() - before) * 1000)
            got = chunk_docs(res)
            top = got[0] if got else None
            ok = top == docs.get(case["top"])
            verdicts.append(
                {
                    "query": case["query"],
                    "expect": case["top"],
                    "got": next((k for k, v in docs.items() if v == top), top),
                    "pass": ok,
                    "top5": [
                        {
                            "file": next((k for k, v in docs.items() if v == d), d),
                            "score": s["score"],
                        }
                        for d, s in zip(got[:5], top_scores(res, 5))
                    ],
                    "ms": ms,
                }
            )
            tr.add(
                "text_to_image:" + case["query"],
                {
                    "POST": "/api/v1/query",
                    "body": {
                        "query": case["query"],
                        "context_only": True,
                        "document_filter": {"doc_kinds": ["image"]},
                    },
                },
                st,
                {"sources": top_scores(res, 4), "ranked": [c["file"] for c in verdicts[-1]["top5"]]},
                ms=ms,
                note="expect " + case["top"],
            )

    print("\n== 5. image -> image")
    for probe in corpus["cross_modal"]:
        blob = (IMAGES / probe["probe"]).read_bytes()
        raw, ctype = multipart(
            [
                (
                    "request",
                    json.dumps(
                        {
                            "query": "",
                            "context_only": True,
                            "document_filter": {"doc_kinds": ["image"]},
                        }
                    ),
                )
            ],
            [("query_image", Path(probe["probe"]).name, blob)],
        )
        before = time.time()
        st, res = call(base, "POST", "/api/v1/query", raw=raw, ctype=ctype, headers=h)
        ms = int((time.time() - before) * 1000)
        got = chunk_docs(res)
        top = got[0] if got else None
        top_file = next((k for k, v in docs.items() if v == top), top)
        accept = probe["accept"]
        ok = top_file in accept
        verdicts.append(
            {
                "probe": probe["probe"],
                "caption": probe.get("caption"),
                "why": probe["why"],
                "accept": accept,
                "got": top_file,
                "pass": ok,
                "top5": [
                    {"file": next((k for k, v in docs.items() if v == d), d), "score": s["score"]}
                    for d, s in zip(got[:5], top_scores(res, 5))
                ],
                "ms": ms,
            }
        )
        tr.add(
            "image_to_image:" + Path(probe["probe"]).name,
            {"POST": "/api/v1/query", "multipart": {"query_image": probe["probe"], "query": ""}},
            st,
            {"sources": top_scores(res, 4), "ranked": [v["file"] for v in verdicts[-1]["top5"]]},
            ms=ms,
            note=probe["why"],
        )

    print("\n== 6. refusal on a text-only workspace")
    st, text_ws = call(
        base,
        "POST",
        f"/api/v1/tenants/{tenant_id}/workspaces",
        body={
            "name": f"Text only {suffix}",
            "rag_profile": "normal",
            "embedding_modalities": "text",
            # WHY the bare id: the catalog exposes `embeddinggemma`, not `embeddinggemma:latest`.
            # The Ollama tag is a runtime detail; the workspace stores the catalog id.
            "embedding_provider": "ollama",
            "embedding_model": "embeddinggemma",
            "llm_provider": "ollama",
            "llm_model": "gemma4:latest",
        },
        headers={"X-User-ID": USER},
    )
    if st == 201:
        th = {"X-Tenant-ID": tenant_id, "X-User-ID": USER, "X-Workspace-ID": text_ws["id"]}
        raw, ctype = multipart(
            [("request", json.dumps({"query": "x", "context_only": True}))],
            [("query_image", "probe-cat.jpg", (IMAGES / "probes/probe-cat.jpg").read_bytes())],
        )
        st2, res2 = call(base, "POST", "/api/v1/query", raw=raw, ctype=ctype, headers=th)
        tr.add(
            "image_query_refused",
            {"POST": "/api/v1/query", "workspace": "text-only", "multipart": {"query_image": "probe-cat.jpg"}},
            st2,
            {"code": res2.get("code"), "message": res2.get("message")},
            note="fail closed, not a silent text-only fallback",
        )

    print("\n== 7. profile honesty")
    st, graph = call(base, "GET", "/api/v1/graph", headers=h)
    tr.add(
        "graph_refused_on_normal",
        {"GET": "/api/v1/graph"},
        st,
        {"code": graph.get("code")},
        note="a chunk index has no graph to draw, and says so",
    )

    passed = sum(1 for v in verdicts if v["pass"])
    summary = {
        "base": base,
        "tenant_id": tenant_id,
        "workspace_id": ws_id,
        "documents": docs,
        "captions": names,
        "retrieval": verdicts,
        "passed": passed,
        "total": len(verdicts),
        "ingest_status_counts": {
            s: list(statuses.values()).count(s) for s in set(statuses.values())
        },
        "wall_clock_s": round(time.time() - t0, 1),
    }
    (HERE.parent / "api" / "verdict.json").write_text(
        json.dumps(summary, indent=2, ensure_ascii=False)
    )
    tr.dump(HERE.parent / "api" / "transcript.json")

    print(f"\n{passed}/{len(verdicts)} retrieval checks passed in {summary['wall_clock_s']}s")
    for v in verdicts:
        if not v["pass"]:
            want = v.get("query") or v.get("probe")
            print(f"  FAIL {want}: want {v.get('accept') or v.get('expect')} got {v['got']}")
    return 0 if passed == len(verdicts) else 1


if __name__ == "__main__":
    sys.exit(main())
