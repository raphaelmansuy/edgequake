#!/usr/bin/env python3
"""SPEC-180 live check: a Normal workspace that embeds text and images with
Vertex `gemini-embedding-2`, against a running backend and the real Vertex API.

Usage:  python3 live_gemini_e2e.py [http://localhost:8090]

Needs the backend started with GOOGLE_CLOUD_PROJECT set to a project whose account may
call `aiplatform` (the token comes from gcloud on the server; nothing is stored).
It creates a throwaway tenant and prints PASS/FAIL per check. Exit code 1 on any FAIL.
"""
import io
import json
import sys
import time
import urllib.error
import urllib.request
import uuid

from PIL import Image, ImageDraw, ImageFont

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:8090"
USER = "00000000-0000-0000-0000-000000000001"
results = []


def check(name, ok, detail=""):
    results.append(ok)
    print(f"{'PASS' if ok else 'FAIL'}  {name}" + (f"  [{detail}]" if detail else ""))


def call(method, path, body=None, headers=None, raw=None, content_type="application/json"):
    data = raw if raw is not None else (json.dumps(body).encode() if body is not None else None)
    req = urllib.request.Request(BASE + path, data=data, method=method)
    if data is not None:
        req.add_header("Content-Type", content_type)
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    try:
        with urllib.request.urlopen(req, timeout=180) as r:
            return r.status, json.loads(r.read() or b"null")
    except urllib.error.HTTPError as e:
        text = e.read().decode(errors="replace")
        try:
            return e.code, json.loads(text)
        except ValueError:
            return e.code, {"raw": text[:300]}


def multipart(parts):
    boundary = "----live" + uuid.uuid4().hex
    out = b""
    for name, value, filename in parts:
        out += f"--{boundary}\r\n".encode()
        if filename:
            out += (
                f'Content-Disposition: form-data; name="{name}"; filename="{filename}"\r\n'
                "Content-Type: application/octet-stream\r\n\r\n"
            ).encode() + value + b"\r\n"
        else:
            out += f'Content-Disposition: form-data; name="{name}"\r\n\r\n{value}\r\n'.encode()
    out += f"--{boundary}--\r\n".encode()
    return out, f"multipart/form-data; boundary={boundary}"


def picture(word, bg, size=(512, 384)):
    """A plain picture whose only content is a big word on a colored ground."""
    image = Image.new("RGB", size, bg)
    draw = ImageDraw.Draw(image)
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 110)
    except OSError:
        font = ImageFont.load_default()
    draw.text((size[0] // 2, size[1] // 2), word, fill=(20, 20, 20), font=font, anchor="mm")
    buf = io.BytesIO()
    image.save(buf, "PNG")
    return buf.getvalue()


def wait_done(headers, doc_id):
    deadline = time.time() + 120
    while time.time() < deadline:
        _, doc = call("GET", f"/api/v1/documents/{doc_id}", headers=headers)
        status = (doc or {}).get("status", "unknown")
        if status in ("completed", "failed", "partial_failure", "cancelled"):
            return status, doc
        time.sleep(1)
    return "timeout", {}


def top_documents(response, kind=None):
    sources = [s for s in response.get("sources", []) if s.get("source_type") == "chunk"]
    return [s.get("document_id") for s in sources]


def main():
    suffix = uuid.uuid4().hex[:8]
    st, tenant = call("POST", "/api/v1/tenants", {"name": f"gemini live {suffix}"})
    check("tenant created", st == 201, str(st))
    tenant_id = tenant["id"]

    st, ws = call(
        "POST",
        f"/api/v1/tenants/{tenant_id}/workspaces",
        {
            "name": f"gemini images {suffix}",
            "rag_profile": "normal",
            "embedding_modalities": "text,image",
            "embedding_provider": "vertexai",
            "embedding_model": "gemini-embedding-2",
            "embedding_dimension": 1536,
            "llm_provider": "mistral",
            "llm_model": "mistral-small-latest",
        },
        headers={"X-User-ID": USER},
    )
    check("normal text+image workspace created on vertexai/gemini-embedding-2", st == 201, f"{st} {str(ws)[:160]}")
    if st != 201:
        return
    wid = ws["id"]
    h = {"X-Tenant-ID": tenant_id, "X-User-ID": USER, "X-Workspace-ID": wid}
    caps = ws.get("rag_profile_capabilities", {})
    check("capabilities: embeds images, no graph", caps.get("embeds_images") is True and caps.get("extracts_graph") is False, str(caps))

    uploads = {}
    for word, bg in (("BANANA", (250, 220, 40)), ("CAR", (170, 175, 185)), ("TREE", (90, 170, 80))):
        body, ctype = multipart([("file", picture(word, bg), f"{word.lower()}.png")])
        st, doc = call("POST", "/api/v1/documents/upload", raw=body, content_type=ctype, headers=h)
        ok = st in (200, 201, 202)
        check(f"upload {word.lower()}.png accepted", ok, f"{st} {str(doc)[:120]}")
        if ok:
            uploads[word] = doc["document_id"]
    st, doc = call(
        "POST",
        "/api/v1/documents",
        {"title": "Orchard notes", "content": "Bananas ripen faster next to apples. Store them away from direct sun."},
        headers=h,
    )
    check("text document accepted", st in (200, 201, 202), str(st))
    text_doc = doc.get("document_id")

    for label, doc_id in list(uploads.items()) + [("TEXT", text_doc)]:
        status, detail = wait_done(h, doc_id)
        check(f"{label} ingested (chunk-only)", status == "completed", status)

    st, q = call("POST", "/api/v1/query", {"query": "a yellow banana", "context_only": True, "document_filter": {"doc_kinds": ["image"]}}, headers=h)
    docs = top_documents(q) if st == 200 else []
    check("text query finds the BANANA image first", st == 200 and docs[:1] == [uploads.get("BANANA")], f"{st} top={docs[:3]}")
    check("image filter returns only images", st == 200 and text_doc not in docs, f"{docs}")

    st, q = call("POST", "/api/v1/query", {"query": "a car on the road", "context_only": True, "document_filter": {"doc_kinds": ["image"]}}, headers=h)
    docs = top_documents(q) if st == 200 else []
    check("text query finds the CAR image first", st == 200 and docs[:1] == [uploads.get("CAR")], f"{st} top={docs[:3]}")

    body, ctype = multipart(
        [
            ("request", json.dumps({"query": "find this", "context_only": True, "document_filter": {"doc_kinds": ["image"]}}), None),
            ("query_image", picture("BANANA", (255, 235, 90), (400, 300)), "q.png"),
        ]
    )
    st, q = call("POST", "/api/v1/query", raw=body, content_type=ctype, headers=h)
    docs = top_documents(q) if st == 200 else []
    check("image query finds the BANANA image first", st == 200 and docs[:1] == [uploads.get("BANANA")], f"{st} top={docs[:3]}")

    st, q = call("POST", "/api/v1/query", {"query": "how do bananas ripen", "context_only": True}, headers=h)
    docs = top_documents(q) if st == 200 else []
    check("mixed query returns the text note among the hits", st == 200 and text_doc in docs, f"{st} n={len(docs)}")

    st, g = call("GET", "/api/v1/graph", headers=h)
    check("graph is refused on a normal workspace (409)", st == 409, str(st))


if __name__ == "__main__":
    main()
    failed = results.count(False)
    print(f"\n{results.count(True)} passed, {failed} failed")
    sys.exit(1 if failed else 0)
