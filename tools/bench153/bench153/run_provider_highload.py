#!/usr/bin/env python3
"""Provider high-load cards H1–H4 on demo.edgequake.com (SPEC-153 §07)."""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

from bench153 import TENANT_DEMO, WORKSPACE_DEMO, Client, write_json, write_jsonl
from bench153.gcp_inventory import collect_gcp_host
from bench153.highload_cards import run_h1, run_h2, run_h3, run_h4_soak
from bench153.machines import collect_load_generator, collect_sut_from_health, write_machines
from bench153.query_unit import SLO_TOTAL_MS, cold_prompts, knee_from_levels, one_query
from bench153.vm_telemetry import VmTelemetry

PROFILES = {
    "quick": dict(h1=[8, 12], h2=[0.25, 0.5], h2_s=20, h3=[16], h3_n=8, soak_s=0, explore=False),
    "standard": dict(h1=[8, 12, 16, 24, 32], h2=[0.25, 0.5, 1.0, 1.5], h2_s=60, h3=[16, 32], h3_n=16,
                     soak_s=0, explore=False),
    "push": dict(h1=[2, 4, 6, 8, 10, 12, 14, 16, 20, 24, 32],
                 h2=[0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.50, 0.60],
                 h2_s=90, h3=[16, 32], h3_n=32, soak_s=240, explore=True),
}


def _preflight(client: Client, run_dir: Path, base_url: str):
    st, health, _ = client.request("GET", "/health")
    health = health if isinstance(health, dict) else {"body": health}
    write_json(run_dir / "preflight-health.json", health)
    _, ready, _ = client.request("GET", "/ready")
    if st != 200 or (isinstance(ready, dict) and ready.get("ready") is False):
        return None, "demo not healthy/ready"
    probe = one_query(client, cold_prompts(1)[0], "auth-probe", "cold_forced_unique", 1, "preflight")
    if probe.get("http_status") in (401, 403):
        return None, f"BLOCKED auth {probe['http_status']}"
    return health, None


def _observed_llm(units: list[dict]) -> dict:
    real = [u for u in units if not u.get("warmup")]
    return {"providers": sorted({u["llm_provider"] for u in real if u.get("llm_provider")}),
            "models": sorted({u["llm_model"] for u in real if u.get("llm_model")})}


def _soak_concurrency(h1: dict) -> int:
    knee = h1.get("knee") or {}
    return int(knee.get("concurrency") or 8)


def _load_jsonl(path: Path) -> list[dict]:
    return [json.loads(l) for l in path.read_text().splitlines() if l.strip()] if path.exists() else []


def _run_sustain(args, client: Client, run_dir: Path) -> int:
    """Follow-up card on an existing run dir; appends to provider/sustain.json when it already exists."""
    from bench153.sustain import DEFAULT_PLAN, parse_plan, run_ladder

    plan = parse_plan(args.sustain_plan) if args.sustain_plan else DEFAULT_PLAN
    _, err = _preflight(client, run_dir / "sustain-preflight", args.base_url)
    if err:
        print(err)
        return 2
    tele = None if args.no_gcp else VmTelemetry(args.gcp_instance, args.gcp_zone, args.gcp_project)
    if tele:
        tele.start()
    t_run = time.time()
    try:
        levels, units = run_ladder(client, plan, args.soak_s, args.rest_s)
    finally:
        telemetry = tele.stop() if tele else []
    out = run_dir / "provider"
    prev = json.loads((out / "sustain.json").read_text()) if (out / "sustain.json").exists() else None
    write_json(out / "sustain.json", {
        "t_run_start": (prev or {}).get("t_run_start", round(t_run, 2)), "t_run_end": round(time.time(), 2),
        "soak_s": args.soak_s, "rest_s": args.rest_s, "levels": ((prev or {}).get("levels") or []) + levels})
    write_jsonl(out / "units-sustain.jsonl", _load_jsonl(out / "units-sustain.jsonl") + units)
    if telemetry:
        write_jsonl(run_dir / "telemetry-sustain.jsonl", _load_jsonl(run_dir / "telemetry-sustain.jsonl") + telemetry)
    print(json.dumps([(l["label"], l["sustainable"]) for l in levels]))
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", default="https://demo.edgequake.com")
    ap.add_argument("--api-key-file", required=True)
    ap.add_argument("--out", required=True, help="Run directory")
    ap.add_argument("--profile", choices=sorted(PROFILES) + ["sustain"], default="push")
    ap.add_argument("--soak-s", type=float, default=210.0, help="sustain: seconds per level")
    ap.add_argument("--sustain-plan", default="", help="sustain: e.g. closed:10,open:0.6,closed:12")
    ap.add_argument("--rest-s", type=float, default=150.0, help="sustain: idle seconds before each level")
    ap.add_argument("--gcp-project", default="saas-app-001")
    ap.add_argument("--gcp-zone", default="us-central1-a")
    ap.add_argument("--gcp-instance", default="elitizon-db")
    ap.add_argument("--no-gcp", action="store_true", help="Skip GCP inventory + VM telemetry")
    args = ap.parse_args()

    run_dir = Path(args.out)
    (run_dir / "provider").mkdir(parents=True, exist_ok=True)
    key = Path(args.api_key_file).read_text().strip()
    client = Client(base_url=args.base_url, api_key=key, tenant_id=TENANT_DEMO,
                    workspace_id=WORKSPACE_DEMO, timeout_s=180)
    if args.profile == "sustain":
        return _run_sustain(args, client, run_dir)
    health, err = _preflight(client, run_dir, args.base_url)
    if err:
        print(err)
        write_json(run_dir / "provider" / "summary.json", {"result": "BLOCKED", "reason": err})
        return 2

    gcp = {"available": False} if args.no_gcp else collect_gcp_host(
        args.gcp_instance, args.gcp_zone, args.gcp_project)
    write_json(run_dir / "gcp-server.json", gcp)
    load_gen = collect_load_generator()
    sut = collect_sut_from_health(args.base_url, health, TENANT_DEMO, WORKSPACE_DEMO, gcp)
    write_machines(run_dir / "machines.json", load_gen, sut)

    cfg = PROFILES[args.profile]
    tele = None if args.no_gcp else VmTelemetry(args.gcp_instance, args.gcp_zone, args.gcp_project)
    if tele:
        tele.start()
    t_run = time.time()
    try:
        print("H1 fresh questions, closed loop", flush=True)
        h1_levels, h1_units = run_h1(client, cfg["h1"], cfg["explore"])
        h1 = {"levels": h1_levels, "knee": knee_from_levels(h1_levels, "concurrency")}
        print("H2 fresh questions, open loop", flush=True)
        h2_levels, h2_units = run_h2(client, cfg["h2"], cfg["h2_s"], cfg["explore"])
        h2 = {"levels": h2_levels, "knee": knee_from_levels(h2_levels, "lambda_qps")}
        print("H3 FAQ warm", flush=True)
        h3_levels, h3_units = run_h3(client, cfg["h3"], cfg["h3_n"])
        h4, h4_units = None, []
        if cfg["soak_s"]:
            print("H4 soak", flush=True)
            h4, h4_units = run_h4_soak(client, _soak_concurrency(h1), cfg["soak_s"])
    finally:
        telemetry = tele.stop() if tele else []
    all_units = h1_units + h2_units + h3_units + h4_units
    sut["workspace_answer_llm"] = _observed_llm(all_units)
    write_machines(run_dir / "machines.json", load_gen, sut)

    summary = {
        "spec": "153", "protocol": "07-highload-protocol", "card": "provider_highload",
        "profile": args.profile, "base_url": args.base_url, "tenant_id": TENANT_DEMO,
        "workspace_id": WORKSPACE_DEMO, "slo_total_ms": SLO_TOTAL_MS,
        "health_version": health.get("version"), "providers_health": health.get("providers"),
        "H1_cold_closed": h1, "H2_open_loop": h2, "H3_faq_warm": {"levels": h3_levels},
        "H4_soak": h4, "t_run_start": round(t_run, 2), "t_run_end": round(time.time(), 2),
        "wall_s": round(time.time() - t_run, 1), "telemetry_samples": len(telemetry), "result": "PASS",
    }
    out = run_dir / "provider"
    write_json(out / "summary.json", summary)
    write_jsonl(out / "units.jsonl", all_units)
    if telemetry:
        write_jsonl(run_dir / "telemetry.jsonl", telemetry)
    write_json(run_dir / "env.json", {"base_url": args.base_url, "tenant_id": TENANT_DEMO,
               "workspace_id": WORKSPACE_DEMO, "protocol": "07-highload-protocol",
               "profile": args.profile, "machines_path": "machines.json", "gcp_path": "gcp-server.json"})
    print(json.dumps({"H1_knee": h1["knee"].get("concurrency"), "H2_knee": h2["knee"].get("lambda_qps"),
                      "telemetry": len(telemetry)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
