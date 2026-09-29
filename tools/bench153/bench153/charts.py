"""Build every chart for the highload PDF. Returns {name: {png, svg, data_uri}}."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from bench153 import charts_data as data
from bench153 import charts_diagrams as diag


def write_highload_charts(out_dir: Path, summary: dict, units: list[dict], telemetry: list[dict],
                          machines: dict[str, Any], sustain: dict | None = None,
                          sustain_units: list[dict] | None = None,
                          sustain_tele: list[dict] | None = None) -> dict[str, dict[str, str]]:
    d = out_dir / "charts"
    sut = machines.get("sut") or {}
    made = {
        "architecture": diag.architecture(d, machines.get("load_generator") or {},
                                          sut.get("gcp") or {}, sut),
        "ask_pipeline": diag.ask_pipeline(d),
        "loop_models": diag.loop_models(d),
        "phase_timeline": diag.phase_timeline(d, summary),
        "h1_latency": data.h1_latency(d, summary, units),
        "h1_attainment": data.h1_attainment(d, summary),
        "h1_throughput": data.h1_throughput(d, summary),
        "h1_breakdown": data.h1_breakdown(d, summary, telemetry),
        "h2_open_loop": data.h2_open_loop(d, summary),
        "h2_latency": data.h2_latency(d, summary),
        "h3_faq": data.h3_faq(d, summary),
        "h4_soak": data.h4_soak(d, summary, units),
        "server_telemetry": data.server_telemetry(d, summary, telemetry),
    }
    if sustain:
        made["sustain_ladder"] = data.sustain_ladder(d, sustain, sustain_units or [])
        made["sustain_telemetry"] = data.sustain_telemetry(d, sustain, sustain_tele or [])
    return {k: v for k, v in made.items() if v}
