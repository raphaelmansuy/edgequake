"""Read-only VM telemetry streamed over IAP ssh during a run (nothing written on the VM)."""

from __future__ import annotations

import subprocess
import threading
import time
from typing import Any

# One sample every ~4s: epoch, aggregate CPU counters, load average, per-container usage.
_LOOP = (
    "while true; do "
    "echo TS=$(date +%s); head -1 /proc/stat; cat /proc/loadavg; "
    "sudo docker stats --no-stream --format 'CT={{.Name}}|{{.CPUPerc}}|{{.MemUsage}}'; "
    "echo END; sleep 2; done"
)


def _mem_mb(text: str) -> float | None:
    """'152.5MiB / 3.833GiB' -> 152.5 (used, MiB)."""
    used = text.split("/")[0].strip()
    try:
        if used.endswith("GiB"):
            return float(used[:-3]) * 1024
        if used.endswith("MiB"):
            return float(used[:-3])
        if used.endswith("KiB"):
            return float(used[:-3]) / 1024
    except ValueError:
        return None
    return None


def parse_stream(text: str) -> list[dict[str, Any]]:
    """Turn the raw stream into samples with CPU busy% / steal% from counter deltas."""
    samples: list[dict[str, Any]] = []
    cur: dict[str, Any] | None = None
    prev_cpu: list[int] | None = None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("TS="):
            cur = {"ts": int(line[3:]), "containers": {}}
        elif cur is None:
            continue
        elif line.startswith("cpu "):
            cur["cpu"] = [int(x) for x in line.split()[1:9]]
        elif line.startswith("CT="):
            name, cpu, mem = (line[3:].split("|") + ["", ""])[:3]
            try:
                cur["containers"][name] = {
                    "cpu_pct": float(cpu.rstrip("%")), "mem_mb": _mem_mb(mem),
                }
            except ValueError:
                pass
        elif line != "END" and "load1" not in cur and line.count(" ") == 4 and "/" in line:
            cur["load1"] = float(line.split()[0])  # /proc/loadavg: "0.01 0.98 1.20 1/300 4242"
        elif line == "END" and "cpu" in cur:
            c = cur["cpu"]  # user nice system idle iowait irq softirq steal
            if prev_cpu:
                d = [a - b for a, b in zip(c, prev_cpu)]
                total = sum(d) or 1
                cur["cpu_busy_pct"] = round(100 * (total - d[3] - d[4]) / total, 1)
                cur["cpu_steal_pct"] = round(100 * d[7] / total, 1)
                samples.append({k: v for k, v in cur.items() if k != "cpu"})
            prev_cpu = c
            cur = None
    return samples


class VmTelemetry:
    """Background `gcloud compute ssh` reader. Failure is non-fatal (report says 'not collected')."""

    def __init__(self, instance: str, zone: str, project: str):
        self.cmd = [
            "gcloud", "compute", "ssh", instance, f"--zone={zone}", f"--project={project}",
            "--tunnel-through-iap", f"--command={_LOOP}",
        ]
        self.proc: subprocess.Popen | None = None
        self.lines: list[str] = []
        self._t: threading.Thread | None = None

    def start(self) -> None:
        try:
            self.proc = subprocess.Popen(
                self.cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
            )
        except Exception:
            self.proc = None
            return
        self._t = threading.Thread(target=self._pump, daemon=True)
        self._t.start()
        time.sleep(12)  # ssh/IAP warm-up so first load samples are captured

    def _pump(self) -> None:
        assert self.proc and self.proc.stdout
        for line in self.proc.stdout:
            self.lines.append(line)

    def stop(self) -> list[dict[str, Any]]:
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except Exception:
                self.proc.kill()
        return parse_stream("".join(self.lines))
