"""Read-only inventory of the GCP host that serves the demo (no secrets)."""

from __future__ import annotations

import json
import re
import subprocess
from typing import Any

_NOISE = ("NumPy", "WARNING", "increase", "please see")


def _run(cmd: list[str], timeout: int = 90) -> str:
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return out.stdout.strip()
    except Exception:
        return ""


def _gcloud_json(args: list[str], project: str) -> dict[str, Any]:
    raw = _run(["gcloud", *args, f"--project={project}", "--format=json"])
    try:
        return json.loads(raw) if raw else {}
    except json.JSONDecodeError:
        return {}


def _ssh(instance: str, zone: str, project: str, command: str) -> str:
    out = _run(
        [
            "gcloud", "compute", "ssh", instance, f"--zone={zone}", f"--project={project}",
            "--tunnel-through-iap", f"--command={command}",
        ],
        timeout=90,
    )
    return "\n".join(l for l in out.splitlines() if l.strip() and not any(n in l for n in _NOISE))


def _describe_vm(instance: str, zone: str, project: str) -> dict[str, Any]:
    d = _gcloud_json(["compute", "instances", "describe", instance, f"--zone={zone}"], project)
    if not d:
        return {}
    mt = d["machineType"].split("/")[-1]
    mt_d = _gcloud_json(["compute", "machine-types", "describe", mt, f"--zone={zone}"], project)
    nic = (d.get("networkInterfaces") or [{}])[0]
    return {
        "instance": d.get("name"),
        "instance_id": d.get("id"),
        "project": project,
        "zone": zone,
        "machine_type": mt,
        "machine_description": mt_d.get("description"),
        "vcpus": mt_d.get("guestCpus"),
        "memory_mb": mt_d.get("memoryMb"),
        "shared_core": mt_d.get("isSharedCpu"),
        "cpu_platform": d.get("cpuPlatform"),
        "created": d.get("creationTimestamp"),
        "provisioning": (d.get("scheduling") or {}).get("provisioningModel"),
        "vpc": (nic.get("network") or "").split("/")[-1],
        "subnet": (nic.get("subnetwork") or "").split("/")[-1],
        "internal_ip": nic.get("networkIP"),
        "external_ip": ((nic.get("accessConfigs") or [{}])[0]).get("natIP"),
        "labels": d.get("labels"),
        "disks": [
            {"name": x.get("deviceName"), "boot": x.get("boot"), "size_gb": x.get("diskSizeGb")}
            for x in d.get("disks", [])
        ],
    }


def _disk_types(project: str, zone: str, names: list[str]) -> dict[str, str]:
    out: dict[str, str] = {}
    for d in _gcloud_json(["compute", "disks", "list", f"--filter=zone:{zone}"], project) or []:
        if isinstance(d, dict) and d.get("name") in names:
            out[d["name"]] = (d.get("type") or "").split("/")[-1]
    return out


_HOST_CMD = (
    "echo CPU=$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs); "
    "echo NPROC=$(nproc); echo MEM_MB=$(free -m | awk '/Mem:/{print $2}'); "
    "echo KERNEL=$(uname -r); echo OS=$(. /etc/os-release; echo $PRETTY_NAME); "
    "sudo docker ps --format 'CT={{.Names}}|{{.Image}}'; "
    "echo PG=$(sudo docker exec edgequake-postgres psql -U edgequake -d edgequake -Atc "
    "\"show max_connections\" 2>/dev/null)/"
    "$(sudo docker exec edgequake-postgres psql -U edgequake -d edgequake -Atc "
    "\"show shared_buffers\" 2>/dev/null)/"
    "$(sudo docker exec edgequake-postgres psql -U edgequake -d edgequake -Atc "
    "\"show work_mem\" 2>/dev/null)"
)


def _parse_host(text: str) -> dict[str, Any]:
    info: dict[str, Any] = {"containers": []}
    for line in text.splitlines():
        if line.startswith("CT="):
            name, _, image = line[3:].partition("|")
            info["containers"].append({"name": name, "image": image})
        elif "=" in line:
            k, _, v = line.partition("=")
            info[k.lower()] = v.strip()
    pg = info.pop("pg", "")
    parts = pg.split("/")
    if len(parts) == 3 and all(parts):
        info["postgres"] = {
            "max_connections": parts[0], "shared_buffers": parts[1], "work_mem": parts[2],
        }
    return info


def collect_gcp_host(instance: str, zone: str, project: str, ssh: bool = True) -> dict[str, Any]:
    """Best-effort: gcloud describe (+ optional IAP ssh read-only facts). Never raises."""
    vm = _describe_vm(instance, zone, project)
    if not vm:
        return {"available": False, "note": "gcloud describe failed; inventory unavailable"}
    types = _disk_types(project, zone, [d["name"] for d in vm.get("disks", [])] + [instance])
    for d in vm["disks"]:
        d["type"] = types.get(d["name"]) or (types.get(instance) if d.get("boot") else None)
    host: dict[str, Any] = {}
    if ssh:
        host = _parse_host(_ssh(instance, zone, project, _HOST_CMD))
    return {"available": True, "vm": vm, "host": host}


def redact(text: str) -> str:
    return re.sub(r"(?i)(key|token|secret|password)=\S+", r"\1=***", text)
