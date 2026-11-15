#!/usr/bin/env python3
"""Live Mistral journeys against the backend started by make dev-mistral.

1. Health pins: mistral-small-latest and mistral-embed (1024d).
2. Text upload, wait until completed, naive query for a unique fact.
3. PDF upload (edgeparse), wait until completed with markdown, naive query.

Query stats must name mistral / mistral-small-latest and include a source.
"""

from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
import uuid

LLM_MODEL = "mistral-small-latest"
EMBED_MODEL = "mistral-embed"
DEFAULT_TENANT = "00000000-0000-0000-0000-000000000002"
POLL_SECONDS = int(os.environ.get("MISTRAL_E2E_POLL_SECONDS", "600"))
QUERY_TIMEOUT = int(os.environ.get("MISTRAL_E2E_QUERY_TIMEOUT", "180"))


def request(
    method: str,
    url: str,
    body: dict | bytes | None = None,
    headers: dict | None = None,
    timeout: int = 60,
    content_type: str | None = None,
) -> tuple[int, object]:
    data = body if isinstance(body, bytes) else (None if body is None else json.dumps(body).encode())
    req = urllib.request.Request(url, data=data, method=method)
    if content_type:
        req.add_header("Content-Type", content_type)
    elif isinstance(body, dict):
        req.add_header("Content-Type", "application/json")
    for key, value in (headers or {}).items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read().decode()
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode()
        try:
            payload = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            payload = {"raw": raw[:500]}
        return exc.code, payload


def die(message: str) -> None:
    print(f"✗ {message}", file=sys.stderr)
    raise SystemExit(1)


def minimal_pdf(text: str) -> bytes:
    stream = f"BT /F1 12 Tf 72 720 Td ({text}) Tj ET\n".encode("latin-1", errors="replace")
    objects = [
        b"1 0 obj<< /Type /Catalog /Pages 2 0 R >>endobj\n",
        b"2 0 obj<< /Type /Pages /Kids [3 0 R] /Count 1 >>endobj\n",
        b"3 0 obj<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>endobj\n",
        f"4 0 obj<< /Length {len(stream)} >>stream\n".encode() + stream + b"\nendstream\nendobj\n",
        b"5 0 obj<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>endobj\n",
    ]
    out = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for obj in objects:
        offsets.append(len(out))
        out.extend(obj)
    xref = len(out)
    out.extend(f"xref\n0 {len(offsets)}\n".encode())
    out.extend(b"0000000000 65535 f \n")
    for off in offsets[1:]:
        out.extend(f"{off:010d} 00000 n \n".encode())
    out.extend(
        f"trailer<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    )
    return bytes(out)


def assert_health(base: str) -> dict:
    status, body = request("GET", f"{base}/health")
    if status != 200 or not isinstance(body, dict):
        die(f"health HTTP {status}: {body}")
    providers = body.get("providers") or {}
    llm = providers.get("llm") or {}
    embed = providers.get("embedding") or {}
    if llm.get("name") != "mistral" or llm.get("model") != LLM_MODEL:
        die(f"health llm pin {llm} (want mistral/{LLM_MODEL})")
    if embed.get("name") != "mistral" or embed.get("model") != EMBED_MODEL or embed.get("dimension") != 1024:
        die(f"health embedding pin {embed} (want mistral/{EMBED_MODEL} 1024)")
    print("✓ health mistral-small-latest / mistral-embed")
    return body


def create_workspace(base: str, tenant: str) -> str:
    payload = {
        "name": f"Mistral E2E {uuid.uuid4().hex[:8]}",
        "llm_provider": "mistral",
        "llm_model": LLM_MODEL,
        "embedding_provider": "mistral",
        "embedding_model": EMBED_MODEL,
        "embedding_dimension": 1024,
        "vision_llm_provider": "mistral",
        "vision_llm_model": LLM_MODEL,
        "extraction_mode": "llm",
    }
    status, body = request(
        "POST",
        f"{base}/api/v1/tenants/{tenant}/workspaces",
        payload,
        {"X-Tenant-ID": tenant},
    )
    if status not in (200, 201) or not isinstance(body, dict) or not body.get("id"):
        die(f"create workspace HTTP {status}: {body}")
    print(f"✓ workspace {body['id']}")
    return str(body["id"])


def wait_completed(base: str, headers: dict, document_id: str) -> dict:
    deadline = time.time() + POLL_SECONDS
    last = {}
    while time.time() < deadline:
        status, body = request("GET", f"{base}/api/v1/documents/{document_id}", headers=headers)
        if status != 200 or not isinstance(body, dict):
            die(f"document {document_id} HTTP {status}: {body}")
        last = body
        phase = (body.get("display_status") or body.get("status") or "").lower()
        if phase in ("completed", "complete"):
            return body
        if phase in ("failed", "error"):
            die(f"document {document_id} failed: {body.get('error_message') or body}")
        time.sleep(4)
    die(f"document {document_id} not completed within {POLL_SECONDS}s; last={last.get('display_status')}")


def assert_query(base: str, headers: dict, question: str, token: str) -> dict:
    status, body = request(
        "POST",
        f"{base}/api/v1/query",
        {"query": question, "mode": "naive"},
        headers,
        timeout=QUERY_TIMEOUT,
    )
    if status != 200 or not isinstance(body, dict):
        die(f"query HTTP {status}: {body}")
    answer = str(body.get("answer") or "")
    sources = body.get("sources") or []
    stats = body.get("stats") or {}
    if not answer.strip():
        die(f"empty answer for {token}")
    if not sources:
        die(f"no sources for {token}; answer={answer[:240]}")
    if stats.get("llm_provider") != "mistral" or stats.get("llm_model") != LLM_MODEL:
        die(f"query stats {stats.get('llm_provider')}/{stats.get('llm_model')} (want mistral/{LLM_MODEL})")
    if token.lower() not in answer.lower():
        die(f"answer missing {token}: {answer[:400]}")
    return body


def journey_text(base: str, headers: dict, token: str) -> dict:
    content = (
        f"Warehouse note. The courier code for this document is {token}. "
        f"The crate count is 17. No other document uses this code."
    )
    status, body = request(
        "POST",
        f"{base}/api/v1/documents",
        {"content": content, "title": f"mistral-e2e-text-{token}", "async_processing": True},
        headers,
    )
    if status not in (200, 202) or not isinstance(body, dict):
        die(f"text upload HTTP {status}: {body}")
    document_id = str(body.get("document_id") or "")
    if not document_id:
        die(f"text upload missing document_id: {body}")
    doc = wait_completed(base, headers, document_id)
    answer = assert_query(
        base,
        headers,
        f"What is the courier code in the warehouse note? Reply with the code {token} if it is present.",
        token,
    )
    print(f"✓ text upload+query {document_id}")
    return {"document_id": document_id, "status": doc.get("display_status"), "answer": answer.get("answer"), "stats": answer.get("stats")}


def journey_pdf(base: str, headers: dict, token: str) -> dict:
    sentence = f"The dock badge code is {token} and the shift size is 9."
    pdf = minimal_pdf(sentence)
    boundary = "----EdgeQuakeMistralE2E"
    parts = []
    for name, value in (
        ("pdf_parser_backend", "edgeparse"),
        ("title", f"mistral-e2e-pdf-{token}"),
    ):
        parts.append(
            f"--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n".encode()
        )
    parts.append(
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"mistral-e2e-{token}.pdf\"\r\nContent-Type: application/pdf\r\n\r\n".encode()
        + pdf
        + b"\r\n"
    )
    parts.append(f"--{boundary}--\r\n".encode())
    status, body = request(
        "POST",
        f"{base}/api/v1/documents/pdf",
        b"".join(parts),
        headers,
        timeout=120,
        content_type=f"multipart/form-data; boundary={boundary}",
    )
    if status not in (200, 202) or not isinstance(body, dict):
        die(f"pdf upload HTTP {status}: {body}")
    document_id = str(body.get("document_id") or "")
    if not document_id:
        die(f"pdf upload missing document_id: {body}")
    doc = wait_completed(base, headers, document_id)
    markdown = str(doc.get("content") or "")
    if token not in markdown and sentence not in markdown:
        die(f"pdf markdown missing {token}: {markdown[:400]}")
    answer = assert_query(
        base,
        headers,
        f"What is the dock badge code? Include {token} in the answer if the document states it.",
        token,
    )
    print(f"✓ pdf upload+query {document_id}")
    return {"document_id": document_id, "status": doc.get("display_status"), "answer": answer.get("answer"), "stats": answer.get("stats")}


def main() -> None:
    base = os.environ.get("BACKEND_URL", "http://localhost:8090").rstrip("/")
    tenant = os.environ.get("EDGEQUAKE_DEV_TENANT_ID", DEFAULT_TENANT)
    assert_health(base)
    workspace = create_workspace(base, tenant)
    headers = {"X-Tenant-ID": tenant, "X-Workspace-ID": workspace}
    text_token = "MTEXT" + uuid.uuid4().hex[:8].upper()
    pdf_token = "MPDF" + uuid.uuid4().hex[:8].upper()
    summary = {
        "tenant_id": tenant,
        "workspace_id": workspace,
        "text": journey_text(base, headers, text_token),
        "pdf": journey_pdf(base, headers, pdf_token),
    }
    for key in ("text", "pdf"):
        stats = summary[key]["stats"] or {}
        summary[key]["answer"] = str(summary[key]["answer"] or "")[:400]
        summary[key]["llm_provider"] = stats.get("llm_provider")
        summary[key]["llm_model"] = stats.get("llm_model")
        summary[key].pop("stats", None)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
