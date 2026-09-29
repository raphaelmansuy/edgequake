#!/usr/bin/env python3
"""Render plain-English SPEC-153 full workload PDF via WeasyPrint."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


def esc(x) -> str:
    return html.escape("" if x is None else str(x))


def ms_s(ms) -> str:
    if ms is None:
        return "—"
    try:
        v = float(ms)
    except (TypeError, ValueError):
        return "—"
    if v >= 1000:
        return f"{v/1000:.1f} s"
    return f"{v:.0f} ms"


def build_html(summary: dict, run_id: str) -> str:
    biz = summary.get("business") or {}
    provider = summary.get("provider") or {}
    system = summary.get("system") or {}
    scorecard = summary.get("layer_scorecard") or {}
    cold1 = provider.get("cold_c1") or {}
    warm = provider.get("warm") or {}
    stream = provider.get("stream") or {}
    knee = provider.get("knee") or {}
    detail = system.get("detail") or {}
    s1 = detail.get("S1_queue_only") or {}
    s2 = (detail.get("S2_ingest_short") or {}).get("levels") or []
    s3 = (detail.get("S3_ingest_dense") or {}).get("levels") or []
    s4 = detail.get("S4_embed_batch") or {}

    bullets = "".join(f"<li>{esc(b)}</li>" for b in biz.get("bottom_line") or [])
    allowed = "".join(f"<li>{esc(b)}</li>" for b in biz.get("allowed_claims") or [])
    forbidden = "".join(f"<li>{esc(b)}</li>" for b in biz.get("forbidden_claims") or [])

    layer_rows = ""
    for lid in ["L0", "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8"]:
        row = scorecard.get(lid) or {}
        layer_rows += (
            f"<tr><td>{esc(lid)}</td><td>{esc(row.get('status'))}</td>"
            f"<td>{esc(row.get('safe_claim'))}</td></tr>"
        )

    cold_levels = provider.get("cold_levels") or []
    cold_rows = ""
    for lv in cold_levels:
        cold_rows += (
            f"<tr><td class='num'>{esc(lv.get('concurrency'))}</td>"
            f"<td class='num'>{esc(lv.get('n'))}</td>"
            f"<td class='num'>{esc(int((lv.get('oracle_ok_rate') or 0)*100))}%</td>"
            f"<td class='num'>{esc(int((lv.get('attainment') or 0)*100))}%</td>"
            f"<td class='num'>{esc(ms_s(lv.get('total_ms_p50')))}</td>"
            f"<td class='num'>{esc(ms_s(lv.get('total_ms_p90')))}</td>"
            f"<td class='num'>{esc(int((lv.get('answer_cache_hit_rate') or 0)*100))}%</td></tr>"
        )

    s2_rows = ""
    for lv in s2:
        s2_rows += (
            f"<tr><td class='num'>{esc(lv.get('concurrency'))}</td>"
            f"<td class='num'>{esc(lv.get('oracle_ok_rate'))}</td>"
            f"<td class='num'>{esc(lv.get('docs_per_s'))}</td>"
            f"<td class='num'>{esc(ms_s(lv.get('wall_ms_p50')))}</td>"
            f"<td class='num'>{esc(ms_s(lv.get('wall_ms_p90')))}</td></tr>"
        )
    s3_rows = ""
    for lv in s3:
        s3_rows += (
            f"<tr><td class='num'>{esc(lv.get('concurrency'))}</td>"
            f"<td class='num'>{esc(lv.get('docs_per_s'))}</td>"
            f"<td class='num'>{esc(ms_s(lv.get('wall_ms_p50')))}</td></tr>"
        )

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>EdgeQuake Full Workload Report</title>
<style>
  @page {{ size: A4; margin: 16mm 15mm 16mm 15mm;
    @bottom-center {{ content: "EdgeQuake · SPEC-153 Full Workload Report · {esc(run_id)}"; font-size: 8pt; color: #64748b; }}
    @bottom-right {{ content: counter(page); font-size: 8pt; color: #64748b; }}
  }}
  body {{ font-family: Helvetica, Arial, sans-serif; color: #0f172a; font-size: 10.5pt; line-height: 1.45; margin: 0; }}
  h1,h2,h3 {{ font-family: Georgia, "Times New Roman", serif; font-weight: 700; margin: 0 0 0.4em; }}
  h1 {{ font-size: 22pt; line-height: 1.15; }}
  h2 {{ font-size: 13.5pt; margin-top: 1.2em; padding-bottom: 0.25em; border-bottom: 1.5px solid #e2e8f0; }}
  h3 {{ font-size: 11pt; color: #0f766e; margin-top: 0.9em; }}
  p {{ margin: 0 0 0.65em; }}
  .hero {{ background: linear-gradient(135deg,#0f172a 0%,#134e4a 55%,#0f766e 100%); color:#f8fafc; padding:20px 22px; border-radius:6px; margin-bottom:14px; }}
  .hero p {{ color:#cbd5e1; }}
  .eyebrow {{ text-transform:uppercase; letter-spacing:.12em; font-size:8.5pt; color:#99f6e4; font-weight:700; margin-bottom:8px; font-family:Helvetica,Arial,sans-serif; }}
  .meta {{ display:grid; grid-template-columns:1fr 1fr; gap:8px 16px; margin-top:12px; font-size:9.5pt; }}
  .meta div {{ border-top:1px solid rgba(255,255,255,.18); padding-top:6px; }}
  .meta strong {{ display:block; color:#99f6e4; font-size:8pt; text-transform:uppercase; letter-spacing:.06em; }}
  .callout {{ background:#f8fafc; border-left:4px solid #0f766e; padding:10px 12px; margin:12px 0; border-radius:0 4px 4px 0; }}
  .callout.warn {{ border-left-color:#b45309; background:#ffedd5; }}
  .callout.good {{ border-left-color:#166534; background:#dcfce7; }}
  table {{ width:100%; border-collapse:collapse; margin:8px 0 12px; font-size:9.5pt; }}
  th,td {{ border-bottom:1px solid #e2e8f0; padding:7px 8px; text-align:left; vertical-align:top; }}
  th {{ background:#f8fafc; font-size:8.5pt; text-transform:uppercase; letter-spacing:.04em; color:#475569; }}
  td.num, th.num {{ text-align:right; font-variant-numeric:tabular-nums; }}
  .kpi-row {{ display:grid; grid-template-columns:repeat(3,1fr); gap:10px; margin:10px 0; }}
  .kpi {{ background:#f8fafc; border:1px solid #e2e8f0; border-radius:6px; padding:10px 12px; }}
  .kpi .label {{ font-size:8pt; text-transform:uppercase; letter-spacing:.06em; color:#475569; font-weight:700; }}
  .kpi .value {{ font-size:16pt; font-family:Georgia,serif; margin-top:2px; }}
  .kpi .hint {{ font-size:8.5pt; color:#64748b; }}
  ul {{ margin:0.3em 0 0.8em 1.1em; }}
  li {{ margin-bottom:0.3em; }}
  .mono {{ font-family:ui-monospace,Menlo,monospace; font-size:8.5pt; }}
  .footer-note {{ font-size:8.5pt; color:#64748b; margin-top:14px; }}
  .page-break {{ break-before: page; }}
</style>
</head>
<body>
<section class="hero">
  <div class="eyebrow">SPEC-153 · Full hybrid workload report</div>
  <h1>EdgeQuake capacity &amp; workload — plain English</h1>
  <p>A full hybrid measurement: local system ingest with a local language model (Ollama), plus live question-and-answer on the shared demo tenant. Numbers are evidence, not marketing targets.</p>
  <div class="meta">
    <div><strong>Run id</strong>{esc(run_id)}</div>
    <div><strong>Suite</strong>Hybrid (System local + Provider demo)</div>
    <div><strong>Demo URL</strong>{esc(provider.get('base_url') or 'https://demo.edgequake.com')}</div>
    <div><strong>Demo version</strong>{esc(provider.get('health_version'))}</div>
    <div><strong>Tenant</strong><span class="mono">{esc(provider.get('tenant_id'))}</span></div>
    <div><strong>Workspace</strong><span class="mono">{esc(provider.get('workspace_id'))}</span></div>
  </div>
</section>

<div class="callout good">
  <strong>Bottom line</strong>
  <ul>{bullets or "<li>See detailed sections below.</li>"}</ul>
</div>

<h2>1. What we tested</h2>
<p>EdgeQuake turns documents into a searchable knowledge graph, then answers questions. Because answers depend on language models, a single “requests per second” number can mislead. This report follows SPEC-153: measure each service layer, keep <em>fresh</em> answers separate from <em>cached</em> repeats, and say clearly what was not tested.</p>
<table>
  <thead><tr><th>Part</th><th>Where</th><th>What</th></tr></thead>
  <tbody>
    <tr><td>System cards S1–S4</td><td>Local mock backend</td><td>Admit/queue proxy, short &amp; dense ingest, embedding estimate</td></tr>
    <tr><td>Provider P1 cold</td><td>demo.edgequake.com</td><td>Unique questions, concurrency 1→8, 25s comfort target</td></tr>
    <tr><td>Provider P2 warm</td><td>demo.edgequake.com</td><td>Repeated questions (answer cache)</td></tr>
    <tr><td>Provider P3 stream</td><td>demo.edgequake.com</td><td>Streaming path / time-to-first-token signals</td></tr>
  </tbody>
</table>
<div class="callout warn">
  <strong>Not tested on the shared demo:</strong> uploading or re-processing PDFs (ingest L2–L6 with a live vendor). That protects the shared corpus and matches SPEC-153 safety rules. Ingest capacity below is <em>local mock</em> only.
</div>

<h2>2. Demo question experience (live)</h2>
<div class="kpi-row">
  <div class="kpi"><div class="label">Fresh success</div><div class="value">{esc(int((cold1.get('oracle_ok_rate') or 0)*100))}%</div><div class="hint">Unique cold prompts at C=1</div></div>
  <div class="kpi"><div class="label">Fresh typical time</div><div class="value">{esc(ms_s(cold1.get('total_ms_p50')))}</div><div class="hint">Server total_time p50</div></div>
  <div class="kpi"><div class="label">Fresh upper time</div><div class="value">{esc(ms_s(cold1.get('total_ms_p90')))}</div><div class="hint">About 90% of asks faster than this</div></div>
</div>
<table>
  <thead><tr><th class="num">Users at once</th><th class="num">Asks</th><th class="num">Success</th><th class="num">In SLO</th><th class="num">Typical</th><th class="num">Upper</th><th class="num">Cache hits</th></tr></thead>
  <tbody>{cold_rows or "<tr><td colspan='7'>No cold data</td></tr>"}</tbody>
</table>
<p><strong>Warm (repeats):</strong> concurrency 4 · success {esc(int((warm.get('oracle_ok_rate') or 0)*100))}% ·
cache hit rate {esc(int((warm.get('answer_cache_hit_rate') or 0)*100))}% ·
typical {esc(ms_s(warm.get('total_ms_p50')))} · upper {esc(ms_s(warm.get('total_ms_p90')))}.</p>
<p><strong>Streaming:</strong> samples={esc(stream.get('n'))} · TTFT p50={esc(ms_s(stream.get('ttft_ms_p50')))} ·
TTFT p90={esc(ms_s(stream.get('ttft_ms_p90')))}.</p>
<p><strong>Capacity knee:</strong> {esc(knee.get('status'))}
{" — tested up to C=" + esc(knee.get('tested_max_concurrency')) if knee.get('tested_max_concurrency') else ""}.</p>

<div class="page-break"></div>
<h2>3. Local ingest (system / mock model)</h2>
<p>These numbers show how the EdgeQuake pipeline moves documents when the language model is a fast mock. They isolate software and database work from vendor LLM latency.</p>
<h3>Admit / queue proxy (S1)</h3>
<p>{esc(s1.get('n'))} tiny uploads · {esc(s1.get('admits_per_s'))} admits/s · admit p50 {esc(ms_s(s1.get('admit_ms_p50')))} · p90 {esc(ms_s(s1.get('admit_ms_p90')))}.</p>
<h3>Short documents (S2)</h3>
<table>
  <thead><tr><th class="num">Concurrency</th><th class="num">Success rate</th><th class="num">Docs/s</th><th class="num">Typical sojourn</th><th class="num">Upper</th></tr></thead>
  <tbody>{s2_rows or "<tr><td colspan='5'>No data</td></tr>"}</tbody>
</table>
<h3>Dense documents (S3)</h3>
<table>
  <thead><tr><th class="num">Concurrency</th><th class="num">Docs/s</th><th class="num">Typical sojourn</th></tr></thead>
  <tbody>{s3_rows or "<tr><td colspan='3'>No data</td></tr>"}</tbody>
</table>
<h3>Embedding estimate (S4)</h3>
<p>Estimated embedding tokens (chars÷2.5): {esc(s4.get('embed_est'))} ·
approx {esc(s4.get('embed_est_per_s'))} est-tokens/s over ingest wall time ·
oracle_ok={esc(s4.get('oracle_ok'))}. This is an <strong>estimate</strong>, not provider usage.</p>

<h2>4. Layer scorecard</h2>
<table>
  <thead><tr><th>Layer</th><th>Status</th><th>Plain-English claim</th></tr></thead>
  <tbody>{layer_rows}</tbody>
</table>

<h2>5. What stakeholders can say</h2>
<h3>Safe claims</h3>
<ul>{allowed}</ul>
<h3>Avoid saying</h3>
<ul>{forbidden}</ul>

<h2>6. Method appendix</h2>
<ul>
  <li>Protocol: <span class="mono">specs/153-workload-benchmark/</span> (SPEC-153).</li>
  <li>Comfort hypothesis for demo Q&amp;A: total server time ≤ 25 seconds; attainment ≥ 90%.</li>
  <li>Cold mode on hosted demo uses unique prompts because server cache cannot be force-disabled from the client.</li>
  <li>Pass/fail: report card complete and internally consistent — not “hit a marketing tokens/sec.”</li>
  <li>Evidence: <span class="mono">specs/153-workload-benchmark/measurements/{esc(run_id)}/</span></li>
</ul>
<p class="footer-note">Generated by tools/bench153 · WeasyPrint · hybrid suite.</p>
</body></html>"""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--weasyprint", default="weasyprint")
    args = ap.parse_args()
    run = Path(args.run_dir)
    summary = json.loads((run / "summary.json").read_text())
    html_path = run / "full-report.html"
    pdf_path = run / "EdgeQuake-Full-Workload-Report.pdf"
    html_path.write_text(build_html(summary, run.name))
    import subprocess

    cmd = [args.weasyprint, str(html_path), str(pdf_path)]
    # try venv weasyprint first via python -m
    try:
        subprocess.check_call(
            [
                "/Users/raphaelmansuy/.venv/bin/weasyprint",
                str(html_path),
                str(pdf_path),
            ]
        )
    except Exception:
        subprocess.check_call(cmd)
    print(pdf_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
