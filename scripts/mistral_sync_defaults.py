#!/usr/bin/env python3
"""Pin server llm-defaults, the default tenant, and the default workspace to Mistral.

LLM and vision use mistral-small-latest. Embeddings use mistral-embed (1024d).
Workspace rows override server defaults, so a reused database is updated too.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request

LLM_MODEL = "mistral-small-latest"
EMBED_MODEL = "mistral-embed"
EMBED_DIM = 1024
PROVIDER = "mistral"
DEFAULT_TENANT = "00000000-0000-0000-0000-000000000002"
DEFAULT_WORKSPACE = "00000000-0000-0000-0000-000000000003"

PINS = {
    "llm_provider": PROVIDER,
    "llm_model": LLM_MODEL,
    "embedding_provider": PROVIDER,
    "embedding_model": EMBED_MODEL,
    "vision_provider": PROVIDER,
    "vision_model": LLM_MODEL,
}

TENANT_PINS = {
    "default_llm_provider": PROVIDER,
    "default_llm_model": LLM_MODEL,
    "default_embedding_provider": PROVIDER,
    "default_embedding_model": EMBED_MODEL,
    "default_embedding_dimension": EMBED_DIM,
    "default_vision_llm_provider": PROVIDER,
    "default_vision_llm_model": LLM_MODEL,
}

WORKSPACE_PINS = {
    "llm_provider": PROVIDER,
    "llm_model": LLM_MODEL,
    "embedding_provider": PROVIDER,
    "embedding_model": EMBED_MODEL,
    "embedding_dimension": EMBED_DIM,
    "vision_llm_provider": PROVIDER,
    "vision_llm_model": LLM_MODEL,
    "extraction_mode": "llm",
}


def request(method: str, url: str, body: dict | None = None, headers: dict | None = None) -> tuple[int, object]:
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Accept", "application/json")
    if body is not None:
        req.add_header("Content-Type", "application/json")
    for key, value in (headers or {}).items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            raw = resp.read().decode()
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode()
        try:
            payload = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            payload = {"raw": raw[:400]}
        return exc.code, payload


def fail(strict: bool, message: str) -> None:
    print(f"✗ {message}", file=sys.stderr)
    if strict:
        raise SystemExit(1)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default=os.environ.get("BACKEND_URL", "http://localhost:8090"))
    parser.add_argument("--strict", action="store_true", help="Exit 1 if any pin write fails")
    args = parser.parse_args()
    base = args.base_url.rstrip("/")
    tenant = os.environ.get("EDGEQUAKE_DEV_TENANT_ID", DEFAULT_TENANT)
    workspace = os.environ.get("EDGEQUAKE_DEV_WORKSPACE_ID", DEFAULT_WORKSPACE)
    headers = {"X-Tenant-ID": tenant, "X-Workspace-ID": workspace}

    status, body = request("PATCH", f"{base}/api/v1/settings/llm-defaults", PINS, headers)
    if status != 200:
        fail(args.strict, f"PATCH llm-defaults HTTP {status}: {body}")
    else:
        print("✓ llm-defaults → mistral-small-latest / mistral-embed")

    status, body = request("PUT", f"{base}/api/v1/tenants/{tenant}", TENANT_PINS, headers)
    if status != 200:
        fail(args.strict, f"PUT tenant {tenant} HTTP {status}: {body}")
    else:
        print(f"✓ tenant {tenant} → mistral-small-latest / mistral-embed")

    status, listed = request("GET", f"{base}/api/v1/tenants/{tenant}/workspaces", headers=headers)
    items = listed.get("items", []) if isinstance(listed, dict) else []
    targets: dict[str, str] = {}
    if status == 200:
        for item in items:
            wid = str(item.get("id") or "")
            slug = str(item.get("slug") or "")
            name = str(item.get("name") or "Default")
            if wid == workspace or slug == "default":
                targets[wid] = name
    else:
        fail(args.strict, f"GET workspaces HTTP {status}: {listed}")

    if workspace not in targets:
        status, current = request("GET", f"{base}/api/v1/workspaces/{workspace}", headers=headers)
        if status == 200 and isinstance(current, dict):
            targets[workspace] = str(current.get("name") or "Default")
        else:
            print(f"• default workspace {workspace} not present; skipped")

    for wid, name in targets.items():
        payload = {"name": name, **WORKSPACE_PINS}
        status, body = request("PUT", f"{base}/api/v1/workspaces/{wid}", payload, headers)
        if status != 200:
            fail(args.strict, f"PUT workspace {wid} HTTP {status}: {body}")
        else:
            print(f"✓ workspace {wid} ({name}) → mistral-small-latest / mistral-embed")


if __name__ == "__main__":
    main()
