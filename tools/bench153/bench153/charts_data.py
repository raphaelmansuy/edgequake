"""Data charts for the highload PDF (all from measured units / summary / telemetry)."""

from __future__ import annotations

from typing import Any

from bench153.charts_style import (
    AMBER, GREEN, NAVY, PURPLE, RED, SKY, SKY_DK, SLATE, TEAL, TEAL_DK, save, secs, setup,
)

SLO_S = 25


def _levels(summary, key):
    return (summary.get(key) or {}).get("levels") or []


def _pctl(xs, p):
    import numpy as np

    return float(np.percentile(xs, p)) if xs else 0.0


def _mark_knee(ax, xs_labels, knee_val, key="concurrency", label="comfort limit"):
    """Shade the labelled category positions up to and including the knee."""
    if knee_val is None or knee_val not in xs_labels:
        return
    i = xs_labels.index(knee_val)
    ax.axvspan(-0.5, i + 0.5, color=GREEN, alpha=0.07, zorder=0)
    ax.text(i + 0.5, ax.get_ylim()[1], f"◀ {label}", ha="right", va="top", fontsize=8,
            color=GREEN, fontweight="bold")


def h1_latency(charts_dir, summary, units):
    plt = setup()
    lv = _levels(summary, "H1_cold_closed")
    cs = [l["concurrency"] for l in lv]
    by_c = {c: [secs(u.get("service_ms") or u["wall_ms"]) for u in units
                if u.get("card") == "H1" and u.get("concurrency") == c and not u.get("warmup")] for c in cs}
    fig, ax = plt.subplots(figsize=(10, 3.4))
    pos = list(range(len(cs)))
    for i, c in enumerate(cs):
        xs = by_c[c]
        if not xs:
            continue
        q10, q25, q50, q75, q90 = (_pctl(xs, p) for p in (10, 25, 50, 75, 90))
        col = TEAL if q90 <= SLO_S else AMBER
        ax.vlines(i, q10, q90, color=col, lw=1.6)
        ax.bar(i, q75 - q25, bottom=q25, width=0.55, color=col, alpha=0.55, ec=col)
        ax.hlines(q50, i - 0.27, i + 0.27, color=NAVY, lw=2.4)
        ax.scatter([i] * len(xs), xs, s=10, color=NAVY, alpha=0.28, zorder=5)
        ax.text(i, q90 + 0.9, f"{q50:.0f}s", ha="center", fontsize=8, color=NAVY, fontweight="bold")
    ax.axhline(SLO_S, color=RED, ls="--", lw=1.6)
    ax.text(len(cs) - 0.5, SLO_S + 0.6, "25 s comfort target", ha="right", color=RED, fontsize=8.5, fontweight="bold")
    ax.set_xticks(pos, [str(c) for c in cs]), ax.set_xlabel("simultaneous people asking (closed loop)")
    ax.set_ylabel("time to get an answer (seconds)"), ax.set_ylim(0, None)
    ax.grid(axis="y"), ax.set_axisbelow(True)
    _mark_knee(ax, cs, (summary["H1_cold_closed"].get("knee") or {}).get("concurrency"))
    ax.set_title("How long people wait as the crowd grows", loc="left")
    return save(fig, plt, charts_dir, "h1_latency")


def h1_attainment(charts_dir, summary):
    plt = setup()
    lv = _levels(summary, "H1_cold_closed")
    cs = [l["concurrency"] for l in lv]
    att = [100 * l["attainment"] for l in lv]
    fig, ax = plt.subplots(figsize=(10, 3.3))
    bars = ax.bar([str(c) for c in cs], att, color=[TEAL if a >= 90 else AMBER for a in att], ec="white", width=0.66)
    ax.axhline(90, color=RED, ls="--", lw=1.6)
    ax.text(len(cs) - 0.5, 91.5, "90% success line", ha="right", color=RED, fontsize=8.5, fontweight="bold")
    for b, a in zip(bars, att):
        ax.text(b.get_x() + b.get_width() / 2, a + 1.5, f"{a:.0f}%", ha="center", fontsize=8.5, fontweight="bold")
    ax.set_ylim(0, 112), ax.set_ylabel("% answered within 25 s"), ax.set_xlabel("simultaneous people asking")
    ax.grid(axis="y"), ax.set_axisbelow(True)
    _mark_knee(ax, cs, (summary["H1_cold_closed"].get("knee") or {}).get("concurrency"))
    ax.set_title("Share of questions answered in time", loc="left")
    return save(fig, plt, charts_dir, "h1_attainment")


def h1_throughput(charts_dir, summary):
    plt = setup()
    lv = _levels(summary, "H1_cold_closed")
    cs = [l["concurrency"] for l in lv]
    fig, (a, b) = plt.subplots(1, 2, figsize=(10, 3.6))
    a.plot(cs, [l["goodput_qps"] for l in lv], "o-", color=TEAL_DK, lw=2.4, ms=6)
    a.fill_between(cs, [l["goodput_qps"] for l in lv], color=TEAL, alpha=0.15)
    a.set_title("Good answers per second", loc="left"), a.set_xlabel("simultaneous people"), a.grid()
    b.plot(cs, [l.get("tokens_per_s") or 0 for l in lv], "s-", color=PURPLE, lw=2.4, ms=6)
    b.fill_between(cs, [l.get("tokens_per_s") or 0 for l in lv], color=PURPLE, alpha=0.12)
    b.set_title("LLM tokens produced per second", loc="left"), b.set_xlabel("simultaneous people"), b.grid()
    fig.tight_layout(w_pad=3)
    return save(fig, plt, charts_dir, "h1_throughput")


def _cpu_mean(telemetry, a, b):
    xs = [t["cpu_busy_pct"] for t in telemetry if a <= t["ts"] <= b]
    return sum(xs) / len(xs) if xs else None


def h1_breakdown(charts_dir, summary, telemetry=()):
    plt = setup()
    lv = [l for l in _levels(summary, "H1_cold_closed") if l.get("generation_ms_mean") is not None]
    if not lv:
        return None
    parts = [("keyword_ms_mean", "understand (LLM)", AMBER), ("embedding_ms_mean", "embed (API)", "#f59e0b"),
             ("generation_ms_mean", "write answer (LLM)", "#92400e"), ("retrieval_ms_mean", "look up (our VM)", TEAL)]
    fig, ax = plt.subplots(figsize=(10, 3.7))
    bottom = [0.0] * len(lv)
    xs = [str(l["concurrency"]) for l in lv]
    for key, name, col in parts:
        vals = [secs(l.get(key)) for l in lv]
        ax.bar(xs, vals, bottom=bottom, color=col, label=name, ec="white", width=0.66)
        bottom = [b + v for b, v in zip(bottom, vals)]
    ax.set_ylabel("average seconds per answer"), ax.set_xlabel("simultaneous people asking")
    ax.grid(axis="y"), ax.set_axisbelow(True), ax.set_ylim(0, max(bottom) * 1.32)
    cpu = [_cpu_mean(telemetry, l["t_start"], l["t_end"]) for l in lv]
    handles, labels = ax.get_legend_handles_labels()
    if all(c is not None for c in cpu):
        ax2 = ax.twinx()
        ax2.spines["right"].set_visible(True)
        (line,) = ax2.plot(xs, cpu, "o-", color=RED, lw=2.4, ms=6, label="VM CPU busy % (right axis)")
        ax2.set_ylim(0, 110), ax2.set_ylabel("VM CPU busy %", color=RED), ax2.tick_params(axis="y", colors=RED)
        handles.append(line), labels.append(line.get_label())
    ax.legend(handles, labels, ncol=3, loc="upper left", fontsize=8)
    ax.set_title("Where the time goes inside one answer — and how busy the VM is", loc="left")
    return save(fig, plt, charts_dir, "h1_breakdown")


def h2_open_loop(charts_dir, summary):
    plt = setup()
    lv = _levels(summary, "H2_open_loop")
    lam = [l["lambda_qps"] for l in lv]
    lab = [f"{x:g}" for x in lam]
    att = [100 * l["attainment"] for l in lv]
    fig, (a, b) = plt.subplots(1, 2, figsize=(11, 3.6), gridspec_kw={"width_ratios": [1.15, 1]})
    a.bar(lab, att, color=[TEAL if x >= 90 else AMBER for x in att], ec="white", width=0.66)
    a.axhline(90, color=RED, ls="--", lw=1.6)
    for i, x in enumerate(att):
        a.text(i, x + 1.5, f"{x:.0f}%", ha="center", fontsize=8, fontweight="bold")
    a.set_ylim(0, 112), a.set_ylabel("% answered within 25 s"), a.set_xlabel("questions arriving per second (λ)")
    a.set_title("Success as arrivals speed up", loc="left"), a.grid(axis="y"), a.set_axisbelow(True)
    b.plot(lam, lam, "--", color=SLATE, lw=1.4, label="offered (ideal)")
    b.plot(lam, [l["goodput_qps"] for l in lv], "o-", color=TEAL_DK, lw=2.4, label="good answers delivered")
    b.set_xlabel("questions arriving per second (λ)"), b.set_ylabel("per second")
    b.set_title("Offered vs delivered", loc="left"), b.legend(fontsize=8), b.grid()
    fig.tight_layout(w_pad=3)
    return save(fig, plt, charts_dir, "h2_open_loop")


def h2_latency(charts_dir, summary):
    plt = setup()
    lv = _levels(summary, "H2_open_loop")
    lam = [l["lambda_qps"] for l in lv]
    fig, ax = plt.subplots(figsize=(10, 3.3))
    ax.fill_between(lam, [secs(l["total_ms_p50"]) for l in lv], [secs(l["total_ms_p90"]) for l in lv],
                    color=AMBER, alpha=0.22, label="typical → slowest 10%")
    ax.plot(lam, [secs(l["total_ms_p50"]) for l in lv], "o-", color=TEAL_DK, lw=2.4, label="typical (median)")
    ax.plot(lam, [secs(l["total_ms_p90"]) for l in lv], "s--", color=AMBER, lw=2.2, label="slowest 10% start here")
    ax.axhline(SLO_S, color=RED, ls="--", lw=1.6), ax.text(lam[0], SLO_S + 0.8, "25 s target", color=RED, fontsize=8.5)
    ax.set_xlabel("questions arriving per second (λ)"), ax.set_ylabel("seconds")
    ax.legend(fontsize=8, loc="upper left"), ax.grid(), ax.set_ylim(0, None)
    ax.set_title("Answer time as the arrival rate rises", loc="left")
    return save(fig, plt, charts_dir, "h2_latency")


def h3_faq(charts_dir, summary):
    plt = setup()
    lv = _levels(summary, "H3_faq_warm")
    cold = _levels(summary, "H1_cold_closed")
    fig, ax = plt.subplots(figsize=(10, 3.3))
    xs = [str(l["concurrency"]) for l in lv]
    ax.bar([i - 0.2 for i in range(len(lv))], [secs(l["total_ms_p50"]) for l in lv], 0.38, color=SKY,
           label="repeat FAQ — typical")
    ax.bar([i + 0.2 for i in range(len(lv))], [secs(l["total_ms_p90"]) for l in lv], 0.38, color=SKY_DK,
           label="repeat FAQ — slowest 10%")
    if cold:
        ref = secs(cold[0]["total_ms_p50"])
        ax.axhline(ref, color=AMBER, ls="--", lw=1.8)
        ax.text(len(lv) - 0.6, ref + 0.5, f"fresh question, lightly loaded: {ref:.0f} s", ha="right",
                color=AMBER, fontsize=8.5, fontweight="bold")
    for i, l in enumerate(lv):
        ax.text(i, secs(l["total_ms_p90"]) + 0.7, f"{100 * l['answer_cache_hit_rate']:.0f}% from cache",
                ha="center", fontsize=8.2, color=NAVY, fontweight="bold")
    ax.set_xticks(range(len(lv)), xs), ax.set_xlabel("simultaneous people repeating a FAQ")
    ax.set_ylabel("seconds"), ax.legend(fontsize=8, loc="upper left"), ax.grid(axis="y"), ax.set_axisbelow(True)
    ax.set_ylim(0, None)
    ax.set_title("Repeated questions: served from memory, not recomputed", loc="left")
    return save(fig, plt, charts_dir, "h3_faq")


def h4_soak(charts_dir, summary, units):
    plt = setup()
    h4 = summary.get("H4_soak")
    pts = sorted((u["started_at"] - h4["t_start"], secs(u.get("service_ms") or u["wall_ms"]))
                 for u in units if u.get("card") == "H4") if h4 else []
    if not pts:
        return None
    import numpy as np

    t, y = np.array([p[0] for p in pts]), np.array([p[1] for p in pts])
    fig, ax = plt.subplots(figsize=(10, 3.4))
    ax.scatter(t / 60, y, s=16, color=PURPLE, alpha=0.55, label="one answer")
    win = max(4, len(y) // 8)
    if len(y) > win:
        roll = np.convolve(y, np.ones(win) / win, mode="valid")
        ax.plot(t[win - 1:] / 60, roll, color=NAVY, lw=2.6, label=f"rolling average ({win} answers)")
    ax.axhline(SLO_S, color=RED, ls="--", lw=1.6)
    ax.set_xlabel("minutes into the soak"), ax.set_ylabel("seconds"), ax.set_ylim(0, None)
    ax.legend(fontsize=8, loc="upper left"), ax.grid()
    ax.set_title(f"Stability: {h4['concurrency']} people asking non-stop for {h4['duration_s'] / 60:.0f} minutes", loc="left")
    return save(fig, plt, charts_dir, "h4_soak")


def _telemetry_plot(charts_dir, name, title, t0, telemetry, bands):
    plt = setup()
    if len(telemetry) < 5:
        return None
    ts = [(s["ts"] - t0) / 60 for s in telemetry]
    fig, axs = plt.subplots(3, 1, figsize=(11, 6.0), sharex=True)
    a, b, c = axs
    busy = [s["cpu_busy_pct"] for s in telemetry]
    a.fill_between(ts, busy, color=TEAL, alpha=0.3)
    a.plot(ts, busy, color=TEAL_DK, lw=1.8, label="VM CPU busy % (both vCPUs)")
    a.plot(ts, [s["cpu_steal_pct"] for s in telemetry], color=RED, lw=1.6, label="CPU stolen by host %")
    a.set_ylim(0, 100), a.set_ylabel("% of the VM's CPU"), a.legend(fontsize=8, loc="lower right"), a.grid()
    a.set_title(title, loc="left")
    for cname, col in (("edgequake-postgres", SKY_DK), ("edgequake-api", TEAL), ("edgequake-caddy", AMBER)):
        b.plot(ts, [s["containers"].get(cname, {}).get("cpu_pct", 0) for s in telemetry], color=col, lw=1.5,
               label=cname.replace("edgequake-", ""))
    b.set_ylabel("container CPU %\n(docker accounting)"), b.legend(fontsize=8, ncol=3, loc="upper right"), b.grid()
    for cname, col in (("edgequake-api", TEAL), ("edgequake-postgres", SKY_DK)):
        c.plot(ts, [s["containers"].get(cname, {}).get("mem_mb", 0) for s in telemetry], color=col, lw=1.8,
               label=cname.replace("edgequake-", ""))
    c.axhline(4096, color=RED, ls=":", lw=1.2), c.text(ts[0], 4096 * 0.9, "VM RAM 4 GB", color=RED, fontsize=8)
    c.set_ylabel("memory MB"), c.set_xlabel("minutes since start"), c.legend(fontsize=8, loc="center right"), c.grid()
    for ax in axs:
        for start, end, col in bands:
            ax.axvspan((start - t0) / 60, (end - t0) / 60, color=col, alpha=0.07, zorder=0)
    fig.tight_layout()
    return save(fig, plt, charts_dir, name)


def server_telemetry(charts_dir, summary, telemetry):
    bands = []
    for key, col in (("H1_cold_closed", TEAL), ("H2_open_loop", AMBER), ("H3_faq_warm", SKY)):
        lv = (summary.get(key) or {}).get("levels") or []
        if lv:
            bands.append((lv[0]["t_start"], lv[-1]["t_end"], col))
    if summary.get("H4_soak"):
        bands.append((summary["H4_soak"]["t_start"], summary["H4_soak"]["t_end"], PURPLE))
    return _telemetry_plot(charts_dir, "server_telemetry",
                           "The VM while the main sweep ran (read-only samples every ~4 s)",
                           summary["t_run_start"], telemetry, bands)


def sustain_telemetry(charts_dir, sustain, telemetry):
    bands = [(l["t_start"], l["t_end"], TEAL if l["sustainable"] else AMBER) for l in sustain["levels"]]
    return _telemetry_plot(charts_dir, "sustain_telemetry",
                           "The VM during the sustained ladder (shaded = load applied; unshaded = idle rest)",
                           sustain["t_run_start"], telemetry, bands)


def sustain_ladder(charts_dir, sustain, units):
    plt = setup()
    lvls = sustain["levels"]
    n = len(lvls)
    rows = -(-(n + 1) // 3)
    fig, axs = plt.subplots(rows, 3, figsize=(11, 2.8 * rows))
    flat = list(axs.flat)
    burst = 120
    for ax, l in zip(flat, lvls):
        pts = [(u["started_at"] - l["t_start"], secs(u.get("service_ms") or u["wall_ms"]))
               for u in units if u.get("card") == "H5" and l["t_start"] - 1 <= u["started_at"] <= l["t_end"]]
        ok = l["sustainable"]
        col = TEAL if ok else AMBER
        ax.axvspan(0, burst, color=SKY, alpha=0.10, zorder=0)
        ax.scatter([p[0] for p in pts], [p[1] for p in pts], s=14, color=col, alpha=0.7, zorder=3)
        ax.axhline(SLO_S, color=RED, ls="--", lw=1.3)
        ax.set_ylim(0, max(45, max([p[1] for p in pts] or [0]) * 1.1)), ax.set_xlim(0, l["soak_s"] + 5)
        st = l["steady"]
        head = f"{l['label']}  →  " + (f"{100 * st['attainment']:.0f}% on time" if st else "n/a")
        ax.set_title(head, loc="left", fontsize=10.5, color=col if not ok else TEAL_DK)
        ax.text(burst / 2, ax.get_ylim()[1] * 0.93, "burst window", ha="center", fontsize=7.6, color=SKY_DK)
        ax.set_xlabel("seconds into the run"), ax.set_ylabel("answer time (s)"), ax.grid()
    for ax in flat[n:-1]:
        ax.axis("off")
    ax = flat[-1]  # summary bars live in the last cell
    names = [l["label"] for l in lvls]
    vals = [100 * ((l["steady"] or {}).get("attainment") or 0) for l in lvls]
    bars = ax.bar(range(n), vals, color=[TEAL if l["sustainable"] else AMBER for l in lvls], ec="white")
    ax.axhline(90, color=RED, ls="--", lw=1.4)
    ax.set_xticks(range(n), names, rotation=30, ha="right", fontsize=8)
    for b, v in zip(bars, vals):
        ax.text(b.get_x() + b.get_width() / 2, v + 2, f"{v:.0f}%", ha="center", fontsize=8, fontweight="bold")
    ax.set_ylim(0, 112), ax.set_ylabel("% on time (after burst)"), ax.grid(axis="y"), ax.set_axisbelow(True)
    ax.set_title("Sustained success", loc="left", fontsize=10.5)
    fig.tight_layout()
    return save(fig, plt, charts_dir, "sustain_ladder")


def peak_stats(telemetry: list[dict[str, Any]]) -> dict[str, float]:
    if not telemetry:
        return {}
    return {
        "cpu_busy_max": max(s["cpu_busy_pct"] for s in telemetry),
        "cpu_busy_p95": _pctl([s["cpu_busy_pct"] for s in telemetry], 95),
        "steal_max": max(s["cpu_steal_pct"] for s in telemetry),
        "api_mem_max": max(s["containers"].get("edgequake-api", {}).get("mem_mb") or 0 for s in telemetry),
        "pg_mem_max": max(s["containers"].get("edgequake-postgres", {}).get("mem_mb") or 0 for s in telemetry),
        "api_cpu_max": max(s["containers"].get("edgequake-api", {}).get("cpu_pct") or 0 for s in telemetry),
        "pg_cpu_max": max(s["containers"].get("edgequake-postgres", {}).get("cpu_pct") or 0 for s in telemetry),
        "samples": len(telemetry),
    }
