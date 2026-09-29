#!/usr/bin/env python3
"""Render the plain-English SPEC-153 higher-user-workload PDF (charts embedded as vector SVG)."""

from __future__ import annotations

import argparse
import json
import socket
import subprocess
from pathlib import Path

from bench153.charts import write_highload_charts
from bench153.report_css import CSS
from bench153.report_facts import gather
from bench153.report_intro import hero, protocol_section, question_section, server_section
from bench153.report_results import (
    appendix_section, conclusions_section, h1_section, h2_section, h3_section, h4_section, h5_section,
    health_section,
)


def _read_jsonl(path: Path) -> list[dict]:
    if not path.exists():
        return []
    return [json.loads(l) for l in path.read_text().splitlines() if l.strip()]


def _dns(vm: dict) -> dict:
    host = "demo.edgequake.com"
    try:
        ip = socket.gethostbyname(host)
    except OSError:
        ip = None
    return {"host": host, "ip": ip, "match": bool(ip and ip == vm.get("external_ip"))}


def build_html(summary, machines, units, telemetry, charts, run_id, sustain=None) -> str:
    f = gather(summary, units, telemetry, machines, sustain)
    vm = ((machines.get("sut") or {}).get("gcp") or {}).get("vm") or {}
    body = "".join([
        hero(summary, f, run_id, vm), question_section(),
        server_section(charts, machines, _dns(vm)), protocol_section(charts, summary),
        h1_section(charts, f), h2_section(charts, f), h3_section(charts, f), h4_section(charts, f), h5_section(charts, f),
        health_section(charts, f), conclusions_section(f), appendix_section(summary, run_id, 11 if sustain else 10),
    ])
    return (f'<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"/>'
            f"<title>EdgeQuake — Higher User Workload</title><style>{CSS}</style></head><body>{body}</body></html>")


def _write_pdf(html_path: Path, pdf_path: Path, weasyprint: str) -> bool:
    for binary in (weasyprint, "/Users/raphaelmansuy/.venv/bin/weasyprint"):
        try:
            subprocess.run([binary, str(html_path), str(pdf_path)], check=True, capture_output=True)
            return True
        except Exception:
            continue
    try:
        from weasyprint import HTML

        HTML(filename=str(html_path)).write_pdf(str(pdf_path))
        return True
    except Exception as e:  # noqa: BLE001
        print("weasyprint failed:", e)
        return False


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--weasyprint", default="weasyprint")
    args = ap.parse_args()
    run = Path(args.run_dir)
    summary = json.loads((run / "provider" / "summary.json").read_text())
    machines = json.loads((run / "machines.json").read_text())
    units = _read_jsonl(run / "provider" / "units.jsonl")
    telemetry = _read_jsonl(run / "telemetry.jsonl")
    sp = run / "provider" / "sustain.json"
    sustain = json.loads(sp.read_text()) if sp.exists() else None
    charts = write_highload_charts(run, summary, units, telemetry, machines, sustain,
                                   _read_jsonl(run / "provider" / "units-sustain.jsonl"),
                                   _read_jsonl(run / "telemetry-sustain.jsonl"))
    html = build_html(summary, machines, units, telemetry, charts, run.name, sustain)
    (run / "highload-report.html").write_text(html)
    (run / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    pdf = run / "EdgeQuake-Higher-User-Workload-Report.pdf"
    ok = _write_pdf(run / "highload-report.html", pdf, args.weasyprint)
    print(f"PDF written: {pdf}" if ok else "PDF failed")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
