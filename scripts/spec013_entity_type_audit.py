#!/usr/bin/env python3
"""
SPEC-013 / Issue #217 — Audit graph entity types against workspace allow-list.

Usage:
  # Single workspace
  python3 scripts/spec013_entity_type_audit.py --tenant-id T --workspace-id W

  # All tenants/workspaces (API must be running)
  python3 scripts/spec013_entity_type_audit.py --scan-all
  python3 scripts/spec013_entity_type_audit.py --scan-all --json-out /tmp/spec013-audit.json

Exit codes: 0 = clean, 1 = violations found, 2 = usage/API error.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.request
from collections import Counter
from typing import Any


def _request(
    method: str,
    url: str,
    headers: dict[str, str] | None = None,
    body: dict[str, Any] | None = None,
) -> tuple[int, Any]:
    headers = headers or {}
    data = None
    req_headers = dict(headers)
    if body is not None:
        data = json.dumps(body).encode("utf-8")
        req_headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=req_headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            raw = resp.read().decode("utf-8")
            return resp.status, json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read().decode("utf-8")
        try:
            payload = json.loads(raw) if raw else {"error": raw}
        except json.JSONDecodeError:
            payload = {"error": raw}
        return e.code, payload


def fetch_workspace(api: str, tenant_id: str, workspace_id: str) -> dict[str, Any]:
    status, body = _request(
        "GET",
        f"{api}/api/v1/workspaces/{workspace_id}",
        {"X-Tenant-ID": tenant_id},
    )
    if status != 200:
        raise SystemExit(f"workspace GET failed ({status}): {body}")
    return body


def fetch_all_entities(api: str, tenant_id: str, workspace_id: str) -> list[dict[str, Any]]:
    headers = {
        "X-Tenant-ID": tenant_id,
        "X-Workspace-ID": workspace_id,
    }
    page = 1
    items: list[dict[str, Any]] = []
    while True:
        status, body = _request(
            "GET",
            f"{api}/api/v1/graph/entities?page={page}&page_size=100",
            headers,
        )
        if status != 200:
            raise SystemExit(f"list entities failed ({status}): {body}")
        batch = body.get("items") or body.get("entities") or []
        if not isinstance(batch, list):
            raise SystemExit(f"unexpected entities response: {body}")
        items.extend(batch)
        total_pages = int(body.get("total_pages") or 1)
        if page >= total_pages:
            break
        page += 1
    return items


def list_tenants(api: str, max_items: int = 500) -> list[dict[str, Any]]:
    status, body = _request("GET", f"{api}/api/v1/tenants?limit={max_items}")
    if status != 200:
        raise SystemExit(f"list tenants failed ({status}): {body}")
    return body.get("items") or body.get("tenants") or []


def list_workspaces(api: str, tenant_id: str, max_items: int = 500) -> list[dict[str, Any]]:
    status, body = _request(
        "GET",
        f"{api}/api/v1/tenants/{tenant_id}/workspaces?limit={max_items}",
        {"X-Tenant-ID": tenant_id},
    )
    if status != 200:
        raise SystemExit(f"list workspaces failed ({status}): {body}")
    return body.get("items") or body.get("workspaces") or []


def normalize_type(value: str) -> str:
    return value.strip().upper().replace(" ", "_")


def audit_workspace(
    api: str, tenant_id: str, workspace_id: str, workspace_name: str
) -> dict[str, Any]:
    ws = fetch_workspace(api, tenant_id, workspace_id)
    allow_raw = ws.get("entity_types") or []
    allow = {normalize_type(str(t)) for t in allow_raw if str(t).strip()}

    entities = fetch_all_entities(api, tenant_id, workspace_id)
    type_counts: Counter[str] = Counter()
    violations: list[dict[str, str]] = []

    for ent in entities:
        name = str(ent.get("name") or ent.get("entity_name") or ent.get("id") or "?")
        et = ent.get("entity_type") or ent.get("type") or ""
        et_norm = normalize_type(str(et)) if et else "UNKNOWN"
        type_counts[et_norm] += 1
        if allow and et_norm not in allow:
            violations.append(
                {
                    "entity": name,
                    "entity_type": et_norm,
                    "remediation": (
                        "Re-ingest documents or rebuild knowledge graph for this workspace "
                        "after confirming entity_types allow-list. See "
                        "specs/013-fix-issues-05-2026/issue-217/003-historical-cleanup-runbook.md"
                    ),
                }
            )

    return {
        "workspace_id": workspace_id,
        "workspace_name": workspace_name,
        "tenant_id": tenant_id,
        "allow_list": sorted(allow),
        "entity_count": len(entities),
        "type_counts": dict(type_counts.most_common()),
        "violations": violations,
        "violation_count": len(violations),
    }


def scan_all(api: str, max_tenants: int) -> dict[str, Any]:
    tenants = list_tenants(api, max_tenants)
    workspace_reports: list[dict[str, Any]] = []
    total_violations = 0

    for tenant in tenants:
        tenant_id = str(tenant.get("id") or tenant.get("tenant_id") or "")
        if not tenant_id:
            continue
        tenant_name = str(tenant.get("name") or tenant_id)
        try:
            workspaces = list_workspaces(api, tenant_id)
        except SystemExit as e:
            workspace_reports.append(
                {
                    "tenant_id": tenant_id,
                    "tenant_name": tenant_name,
                    "error": str(e),
                    "violation_count": 0,
                }
            )
            continue

        for ws in workspaces:
            ws_id = str(ws.get("id") or ws.get("workspace_id") or "")
            if not ws_id:
                continue
            ws_name = str(ws.get("name") or ws_id)
            try:
                report = audit_workspace(api, tenant_id, ws_id, ws_name)
            except SystemExit as e:
                report = {
                    "workspace_id": ws_id,
                    "workspace_name": ws_name,
                    "tenant_id": tenant_id,
                    "error": str(e),
                    "violation_count": 0,
                }
            total_violations += int(report.get("violation_count") or 0)
            workspace_reports.append(report)

    return {
        "api": api,
        "tenant_count": len(tenants),
        "workspace_reports": workspace_reports,
        "total_violations": total_violations,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit entity types vs workspace allow-list")
    parser.add_argument("--api", default=os.environ.get("EDGEQUAKE_API", "http://localhost:8080"))
    parser.add_argument("--tenant-id", help="Required unless --scan-all")
    parser.add_argument("--workspace-id", help="Required unless --scan-all")
    parser.add_argument(
        "--scan-all",
        action="store_true",
        help="Audit every tenant/workspace (API must be running)",
    )
    parser.add_argument("--max-tenants", type=int, default=200)
    parser.add_argument("--json-out", help="Write full report JSON to path")
    parser.add_argument("--fail-on-empty-allowlist", action="store_true")
    args = parser.parse_args()

    if args.scan_all:
        summary = scan_all(args.api, args.max_tenants)
        if args.json_out:
            with open(args.json_out, "w", encoding="utf-8") as f:
                json.dump(summary, f, indent=2)
        print(json.dumps(summary, indent=2))
        if summary["total_violations"] > 0:
            print(
                f"\nFAIL: {summary['total_violations']} violation(s) across "
                f"{len(summary['workspace_reports'])} workspace report(s)",
                file=sys.stderr,
            )
            return 1
        print("\nOK: no entity-type violations in scanned workspaces.")
        return 0

    if not args.tenant_id or not args.workspace_id:
        print("ERROR: --tenant-id and --workspace-id required (or use --scan-all)", file=sys.stderr)
        return 2

    ws = fetch_workspace(args.api, args.tenant_id, args.workspace_id)
    if not (ws.get("entity_types") or []) and args.fail_on_empty_allowlist:
        print("ERROR: workspace has empty entity_types allow-list", file=sys.stderr)
        return 2

    report = audit_workspace(
        args.api,
        args.tenant_id,
        args.workspace_id,
        str(ws.get("name") or args.workspace_id),
    )

    if args.json_out:
        with open(args.json_out, "w", encoding="utf-8") as f:
            json.dump(report, f, indent=2)

    print(json.dumps(report, indent=2))

    if report["violation_count"] > 0:
        print(
            f"\nFAIL: {report['violation_count']} entity type(s) outside allow-list",
            file=sys.stderr,
        )
        return 1

    print("\nOK: all entity types within workspace allow-list (or graph empty).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
