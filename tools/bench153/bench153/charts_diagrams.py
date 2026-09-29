"""Explanatory diagrams for the PDF: architecture, ask pipeline, load models, timeline."""

from __future__ import annotations

from typing import Any

from bench153.charts_style import (
    AMBER, GREEN, NAVY, PURPLE, RED, SKY, SKY_DK, SLATE, TEAL, TEAL_DK, TEAL_LT, save, setup,
)


def _box(ax, x, y, w, h, fc, ec, title="", sub="", tc=NAVY, sc=SLATE, lw=1.6, fs=9.5, r=1.2, z=3):
    from matplotlib.patches import FancyBboxPatch

    ax.add_patch(FancyBboxPatch((x, y), w, h, boxstyle=f"round,pad=0,rounding_size={r}",
                                fc=fc, ec=ec, lw=lw, zorder=z))
    if title:
        ax.text(x + w / 2, y + h * (0.66 if sub else 0.5), title, ha="center", va="center",
                fontsize=fs, fontweight="bold", color=tc, zorder=z + 1)
    if sub:
        ax.text(x + w / 2, y + h * 0.30, sub, ha="center", va="center", fontsize=fs - 2.2,
                color=sc, zorder=z + 1, linespacing=1.25)


def _arrow(ax, a, b, color=TEAL, label="", lw=2.0, dy=1.6, style="-|>"):
    ax.annotate("", xy=b, xytext=a, zorder=6,
                arrowprops=dict(arrowstyle=style, color=color, lw=lw, shrinkA=0, shrinkB=0))
    if label:
        ax.text((a[0] + b[0]) / 2, (a[1] + b[1]) / 2 + dy, label, ha="center", fontsize=7.6,
                color=color, fontweight="bold", zorder=7)


def architecture(charts_dir, lg: dict[str, Any], gcp: dict[str, Any], sut: dict[str, Any]):
    plt = setup()
    vm, host = gcp.get("vm") or {}, gcp.get("host") or {}
    images = {c["name"]: c["image"].split(":")[-1] for c in host.get("containers", [])}
    fig, ax = plt.subplots(figsize=(12, 5.2))
    ax.set_xlim(0, 122), ax.set_ylim(0, 62), ax.axis("off"), ax.set_facecolor("white")

    _box(ax, 1, 20, 24, 22, TEAL_LT, TEAL, "Load generator",
         f"{lg.get('cpu_model') or 'client'}\n{lg.get('cpu_logical')} cores · {lg.get('mem_gb')} GB\n"
         f"bench153 harness\n(fires the questions)")
    _arrow(ax, (25, 31), (42, 31), TEAL, "HTTPS · public internet")

    _box(ax, 42, 2, 54, 58, "#eff6ff", SKY_DK, lw=1.4, r=2.0, z=1)
    ax.text(44, 57, f"Google Cloud · project {vm.get('project', '?')} · {vm.get('zone', '?')}",
            fontsize=9, fontweight="bold", color=SKY_DK)
    _box(ax, 44.5, 5, 49, 48, "white", NAVY, lw=1.8, r=1.6, z=2)
    ax.text(46, 50, f"VM {vm.get('instance', '?')}  ·  {vm.get('machine_type', '?')}",
            fontsize=10.5, fontweight="bold", color=NAVY)
    disks = " + ".join(f"{d['size_gb']} GB {d.get('type') or ''}".strip() for d in vm.get("disks", []))
    ax.text(46, 46.6, f"{vm.get('vcpus')} shared vCPU (½ physical core) · "
            f"{(vm.get('memory_mb') or 0) / 1024:.0f} GB RAM · {host.get('cpu', vm.get('cpu_platform'))}",
            fontsize=7.8, color=SLATE)
    ax.text(46, 43.8, f"{host.get('os', '')} · disks {disks} · public IP {vm.get('external_ip')}",
            fontsize=7.2, color=SLATE)

    _box(ax, 47, 26, 14, 10, TEAL_LT, TEAL, "Caddy", f"TLS · port 443\n{images.get('edgequake-caddy', '')}")
    _box(ax, 66, 26, 16, 10, TEAL_LT, TEAL, "EdgeQuake API", f"Rust · Axum\nv{images.get('edgequake-api', '?')}")
    _box(ax, 66, 9, 16, 10, TEAL_LT, TEAL, "Postgres",
         f"AGE + pgvector\nmax_conn {(host.get('postgres') or {}).get('max_connections', '?')}")
    _box(ax, 47, 9, 14, 10, "#f1f5f9", "#94a3b8", "Web UI", "not used by\nthe test", sc="#64748b")
    _arrow(ax, (61, 31), (66, 31), TEAL)
    _arrow(ax, (74, 26), (74, 19), TEAL)
    ax.text(75.4, 22.2, "graph + vector\nlookups", fontsize=7, color=TEAL_DK)

    _box(ax, 101, 20, 20, 22, "#fef3c7", AMBER, "Mistral AI API",
         f"{(sut.get('workspace_answer_llm') or {}).get('models', ['mistral-small-latest'])[0]}\n+ embeddings\n(external service)")
    _arrow(ax, (82, 33), (101, 33), AMBER, "HTTPS · LLM calls", dy=1.7)
    ax.text(83, 29, "most of the waiting\nhappens here →", fontsize=7.4, color=AMBER, style="italic")

    for i, (c, t) in enumerate([(TEAL_LT, "components we operate"), ("#fef3c7", "external LLM service"),
                                ("#eff6ff", "Google Cloud boundary")]):
        ax.add_patch(plt.Rectangle((1 + i * 34, 0.6), 2.6, 1.6, fc=c, ec=NAVY, lw=0.8))
        ax.text(4.4 + i * 34, 1.4, t, fontsize=7.6, va="center", color=SLATE)
    ax.set_title("Where the test runs: client → one GCP VM → external LLM", loc="left", pad=6)
    return save(fig, plt, charts_dir, "architecture")


def ask_pipeline(charts_dir):
    plt = setup()
    fig, ax = plt.subplots(figsize=(12, 3.4))
    ax.set_xlim(0, 120), ax.set_ylim(0, 34), ax.axis("off"), ax.set_facecolor("white")
    steps = [
        ("1  Understand", "pull keywords\nfrom the question", "LLM call", AMBER, "#fef3c7", "keyword_time_ms"),
        ("2  Embed", "turn the question\ninto numbers", "embedding call", AMBER, "#fef3c7", "embedding_time_ms"),
        ("3  Look up", "search the graph +\nvectors in Postgres", "on our VM", TEAL, TEAL_LT, "retrieval_time_ms"),
        ("4  Write answer", "LLM composes the\nreply from sources", "LLM call", AMBER, "#fef3c7", "generation_time_ms"),
        ("5  Return", "answer + stats\nsent to the user", "on our VM", TEAL, TEAL_LT, "total_time_ms"),
    ]
    w, gap = 20.5, 4.4
    for i, (t, s, where, ec, fc, field) in enumerate(steps):
        x = 1 + i * (w + gap)
        _box(ax, x, 12, w, 16, fc, ec, t, s, fs=10)
        ax.text(x + w / 2, 9.8, where, ha="center", fontsize=8, color=ec, fontweight="bold")
        ax.text(x + w / 2, 7.4, field, ha="center", fontsize=6.8, color=SLATE, family="monospace")
        if i < len(steps) - 1:
            _arrow(ax, (x + w, 20), (x + w + gap, 20), NAVY, lw=1.6)
    ax.annotate("", xy=(1, 3.4), xytext=(119, 3.4),
                arrowprops=dict(arrowstyle="<->", color=RED, lw=1.6))
    ax.text(60, 0.6, "A “good” answer must arrive in under 25 seconds end to end (and be non-empty)",
            ha="center", fontsize=8.6, color=RED, fontweight="bold")
    ax.set_title("What happens when one person asks one question", loc="left", pad=6)
    return save(fig, plt, charts_dir, "ask_pipeline")


def loop_models(charts_dir):
    plt = setup()
    fig, (a, b) = plt.subplots(1, 2, figsize=(12, 3.8))
    for ax in (a, b):
        ax.set_xlim(0, 30), ax.set_ylim(-0.5, 4.5), ax.set_yticks([]), ax.set_xlabel("time →")
        ax.spines["left"].set_visible(False)
    a.set_title("Closed loop — a fixed crowd", loc="left")
    durs = [[6, 7, 6, 6], [7, 6, 7, 6], [5, 7, 6, 7], [6, 6, 7, 6]]
    for lane, ds in enumerate(durs):
        x = 0.5
        for k, d in enumerate(ds):
            a.barh(lane, d - 0.3, left=x, height=0.62, color=TEAL if k % 2 == 0 else "#14b8a6", ec="white")
            x += d
        a.text(-0.4, lane, f"user {lane + 1}", ha="right", va="center", fontsize=8, color=SLATE)
    a.text(15, 4.15, "each person asks again the moment the last answer arrives", ha="center",
           fontsize=8.4, color=TEAL_DK, style="italic")
    a.text(15, -1.25, "Always exactly N questions in flight  →  we sweep N (2, 4, 6 … 32)",
           ha="center", fontsize=8.6, color=NAVY, fontweight="bold", clip_on=False)

    b.set_title("Open loop — a steady stream of arrivals", loc="left")
    arrivals = [i * 2.4 + 0.5 for i in range(11)]
    ends: list[float] = []
    for i, t in enumerate(arrivals):
        d = 5 + i * 0.85 if i > 5 else 5.0
        lane = next((k for k, e in enumerate(ends) if e <= t), len(ends))
        if lane == len(ends):
            ends.append(0)
        ends[lane] = t + d
        lane = min(lane, 4)
        b.barh(lane, d - 0.2, left=t, height=0.62, color=AMBER if i > 6 else SKY, ec="white")
        b.annotate("", xy=(t, 4.35), xytext=(t, 4.9), arrowprops=dict(arrowstyle="-|>", color=NAVY, lw=1.1))
    b.set_ylim(-0.5, 5.0)
    b.text(15, -1.25, "Arrivals never wait for earlier answers  →  we sweep the rate λ (0.10 … 0.60 per s)",
           ha="center", fontsize=8.6, color=NAVY, fontweight="bold", clip_on=False)
    b.text(29.5, 3.35, "when arrivals outpace\nthe server, answers\nget slower and stack up", ha="right",
           fontsize=7.8, color=AMBER, style="italic")
    fig.subplots_adjust(wspace=0.12, bottom=0.22)
    return save(fig, plt, charts_dir, "loop_models")


def phase_timeline(charts_dir, summary: dict[str, Any]):
    plt = setup()
    t0 = summary.get("t_run_start") or 0
    rows = [("H1 fresh · crowd", "H1_cold_closed", "concurrency", "C=", TEAL),
            ("H2 fresh · arrivals", "H2_open_loop", "lambda_qps", "λ=", AMBER),
            ("H3 FAQ repeats", "H3_faq_warm", "concurrency", "C=", SKY)]
    fig, ax = plt.subplots(figsize=(12, 3.2))
    for r, (label, key, field, pre, col) in enumerate(rows):
        for lv in (summary.get(key) or {}).get("levels", []):
            s, e = (lv["t_start"] - t0) / 60, (lv["t_end"] - t0) / 60
            ax.barh(r, e - s, left=s, height=0.62, color=col, ec="white", lw=1.2, alpha=0.5 if lv["attainment"] < 0.9 else 1)
            if e - s > 0.55:
                ax.text((s + e) / 2, r, f"{pre}{lv[field]}", ha="center", va="center", fontsize=7.2,
                        color="white", fontweight="bold")
    h4 = summary.get("H4_soak")
    if h4:
        s, e = (h4["t_start"] - t0) / 60, (h4["t_end"] - t0) / 60
        ax.barh(3, e - s, left=s, height=0.62, color=PURPLE, ec="white")
        ax.text((s + e) / 2, 3, f"soak C={h4['concurrency']}", ha="center", va="center", fontsize=7.4,
                color="white", fontweight="bold")
    labels = [r[0] for r in rows] + (["H4 soak (stability)"] if h4 else [])
    ax.set_yticks(range(len(labels)), labels), ax.invert_yaxis(), ax.set_xlabel("minutes since start of run")
    ax.grid(axis="x"), ax.set_axisbelow(True)
    ax.set_title("The test in time — faded blocks = load levels that missed the 90% success line", loc="left")
    return save(fig, plt, charts_dir, "phase_timeline")
