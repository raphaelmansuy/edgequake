"""Collect load-generator + SUT machine inventory for SPEC-153 highload runs."""

from __future__ import annotations

import json
import os
import platform
import socket
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.request import urlopen


def _run(cmd: list[str]) -> str:
    try:
        return subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL).strip()
    except Exception:
        return ""


def _git_sha() -> str | None:
    sha = _run(["git", "rev-parse", "--short", "HEAD"])
    return sha or None


def _public_ip() -> str | None:
    for url in ("https://ifconfig.me/ip", "https://api.ipify.org"):
        try:
            with urlopen(url, timeout=5) as resp:
                ip = resp.read().decode().strip()
                if ip:
                    return ip
        except Exception:
            continue
    return None


def collect_load_generator() -> dict[str, Any]:
    uname = platform.uname()
    cpu_model = _run(["sysctl", "-n", "machdep.cpu.brand_string"]) or platform.processor()
    cpu_logical = _run(["sysctl", "-n", "hw.logicalcpu"]) or str(os.cpu_count() or "")
    mem_bytes = _run(["sysctl", "-n", "hw.memsize"])
    mem_gb = None
    if mem_bytes.isdigit():
        mem_gb = round(int(mem_bytes) / (1024**3), 1)
    sw_vers = {}
    if platform.system() == "Darwin":
        sw_vers = {
            "product_name": _run(["sw_vers", "-productName"]),
            "product_version": _run(["sw_vers", "-productVersion"]),
            "build_version": _run(["sw_vers", "-buildVersion"]),
        }
    return {
        "role": "load_generator",
        "hostname": socket.gethostname(),
        "os": {
            "system": uname.system,
            "release": uname.release,
            "version": uname.version,
            "sw_vers": sw_vers or None,
        },
        "arch": uname.machine,
        "cpu_model": cpu_model or None,
        "cpu_logical": int(cpu_logical) if str(cpu_logical).isdigit() else cpu_logical,
        "mem_gb": mem_gb,
        "python": platform.python_version(),
        "harness": {
            "name": "bench153",
            "version": "0.1.0",
            "git_sha": _git_sha(),
        },
        "network_egress": {
            "public_ip": _public_ip(),
            "note": "client_to_sut: internet_https",
        },
        "timezone": time.tzname,
        "started_at_utc": datetime.now(timezone.utc).isoformat(),
    }


def _hosting_note(gcp: dict[str, Any] | None) -> str:
    vm = (gcp or {}).get("vm")
    if not vm:
        return "GCP host inventory not collected (run with gcloud access to record the exact VM)."
    return (
        f"GCE VM {vm['instance']} ({vm['machine_type']}: {vm['vcpus']} vCPU, "
        f"{(vm['memory_mb'] or 0) / 1024:.0f} GB) in {vm['zone']}, project {vm['project']}"
    )


def collect_sut_from_health(
    base_url: str,
    health: dict[str, Any],
    tenant_id: str,
    workspace_id: str,
    gcp: dict[str, Any] | None = None,
) -> dict[str, Any]:
    schema = health.get("schema") or {}
    caps = schema.get("postgres_capabilities") or {}
    providers = health.get("providers") or {}
    build = health.get("build_info") or {}
    return {
        "role": "sut",
        "base_url": base_url.rstrip("/"),
        "product_version": health.get("version"),
        "build": build,
        "storage_mode": health.get("storage_mode"),
        "postgres_major": caps.get("postgres_major"),
        "pgvector": caps.get("pgvector_version"),
        "age": caps.get("age_version"),
        "fleet_llm": providers.get("llm"),
        "fleet_embed": providers.get("embedding"),
        "tenant_id": tenant_id,
        "workspace_id": workspace_id,
        "hosting_note": _hosting_note(gcp),
        "gcp": gcp or {"available": False},
        "auth": "X-API-Key (secret not stored)",
        "task_queue": (health.get("operational") or {}).get("task_queue"),
    }


def write_machines(path: Path, load_gen: dict, sut: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "spec": "153",
                "protocol": "07-highload-protocol",
                "load_generator": load_gen,
                "sut": sut,
            },
            indent=2,
        )
        + "\n"
    )
