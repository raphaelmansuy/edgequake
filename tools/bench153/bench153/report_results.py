"""Back half of the report: results, server health, what it means, reproduce, glossary."""

from __future__ import annotations

from bench153.query_unit import failing
from bench153.report_util import esc, fig, ms_s, pct


def _verdict_row(lv: dict, first: str, cells: str) -> str:
    ok = not failing(lv)
    return f"<tr class='{'ok' if ok else 'bad'}'>{cells}<td>{'✔ comfortable' if ok else '✖ over the limit'}</td></tr>"


def _h1_table(f: dict) -> str:
    body = "".join(_verdict_row(l, "", (
        f"<td class='num'>{esc(l['concurrency'])}</td><td class='num'>{esc(l['n'])}</td>"
        f"<td class='num'>{pct(l['attainment'])}</td><td class='num'>{ms_s(l['total_ms_p50'])}</td>"
        f"<td class='num'>{ms_s(l['total_ms_p90'])}</td><td class='num'>{l['goodput_qps']:.2f}</td>"
        f"<td class='num'>{pct(l['error_rate'])}</td>")) for l in f["h1"])
    return ("<table><thead><tr><th class='num'>People at once</th><th class='num'>Questions</th>"
            "<th class='num'>On time</th><th class='num'>Typical wait</th><th class='num'>Slowest 10% from</th>"
            "<th class='num'>Good answers/s</th><th class='num'>Errors</th><th>Verdict</th></tr></thead>"
            f"<tbody>{body}</tbody></table>")


def h1_section(charts: dict, f: dict) -> str:
    kl1, sp = f["kl1"], f["split_knee"]
    drop = f["best_goodput"]
    words = (f"Up to <b>{esc(f['kC'])} people</b> asking at the same instant, at least 90% of fresh questions were answered within 25 seconds"
             + (f" (typical wait {ms_s(kl1['total_ms_p50'])}, slowest-tenth starting at {ms_s(kl1['total_ms_p90'])})" if kl1 else "")
             + (f". The first level to miss was <b>{esc(f['fail_C'])}</b>." if f["fail_C"]
                else f". No level up to {esc(f['max_C'])} people missed the target."))
    tail = " Beyond that, more people mostly added waiting rather than more answers." if drop and drop["concurrency"] < f["max_C"] else ""
    goodput = (f" Good answers per second peaked at {drop['goodput_qps']:.2f}/s with {esc(drop['concurrency'])} people at once.{tail}"
               if drop else "")
    llm = (f" At the burst limit an answer spends about {sp['llm_s']:.1f} s in the external-LLM steps and {sp['vm_s']:.1f} s in the lookup on our own VM (Figure 7)." if sp else "")
    return f"""<h2 class="pb">4. Result 1 — a crowd of people asking fresh questions (H1)</h2>
<div class="callout"><b>In one sentence.</b> {words}{goodput}{llm}</div>
{fig(charts, 'h1_latency', "<b>Figure 5 — Waiting time by crowd size.</b> Each grey dot is one real answer. The bar spans the middle half of answers, the thin line spans the fastest to slowest tenth, and the thick tick is the typical (median) wait. Green shading marks the levels that stay within the promise.")}
{fig(charts, 'h1_attainment', "<b>Figure 6 — Share answered in time.</b> Teal bars pass the 90% line; amber bars miss it.")}
{_h1_table(f)}
{fig(charts, 'h1_breakdown', "<b>Figure 7 — Where the time goes.</b> Amber/brown = calls to the external LLM and embedding service, teal = the lookup on our own VM, red line = how busy the VM’s CPU was. If the amber part dominated, a faster VM would not help; if the teal part grows together with the red line, the VM is the limit.")}
{fig(charts, 'h1_throughput', "<b>Figure 8 — Useful output.</b> Left: good answers delivered per second. Right: tokens (word-pieces) generated per second by the LLM. When these flatten, the system is saturated no matter how many more people join.")}"""


def _h2_table(f: dict) -> str:
    body = "".join(_verdict_row(l, "", (
        f"<td class='num'>{l['lambda_qps']:g}/s ({l['lambda_qps'] * 60:.0f}/min)</td>"
        f"<td class='num'>{esc(l.get('offered_count'))}</td><td class='num'>{esc(l['n'])}</td>"
        f"<td class='num'>{pct(l['attainment'])}</td><td class='num'>{ms_s(l['total_ms_p50'])}</td>"
        f"<td class='num'>{ms_s(l['total_ms_p90'])}</td><td class='num'>{pct(l['error_rate'])}</td>")) for l in f["h2"])
    return ("<table><thead><tr><th class='num'>Arrival rate</th><th class='num'>Sent</th><th class='num'>Finished</th>"
            "<th class='num'>On time</th><th class='num'>Typical wait</th><th class='num'>Slowest 10% from</th>"
            "<th class='num'>Errors</th><th>Verdict</th></tr></thead>"
            f"<tbody>{body}</tbody></table>")


def _people_rate(f: dict) -> tuple[float | None, float, str]:
    """(rate/s, typical wait ms, label): prefer the *sustained* rate; else the short-window H2 rate."""
    sus = f["sustain"]
    if sus and sus["best_open"]:
        lvl = next(l for l in sus["levels"] if l["kind"] == "open" and l["value"] == sus["best_open"] and l["sustainable"])
        return sus["best_open"], (lvl["steady"] or {}).get("total_ms_p50") or 0, "sustained"
    return f["kLam"], (f["kl2"] or {}).get("total_ms_p50") or 0, "short-window"


def _users_table(f: dict) -> str:
    lam, ans, label = _people_rate(f)
    if not lam:
        return ""
    rows_ = "".join(
        f"<tr><td>{name}</td><td class='num'>{think}s</td>"
        f"<td class='num'>≈ {lam * (think + ans / 1000):.0f} people</td></tr>"
        for name, think in (("Very active (asks a question every ~30 s)", 30),
                            ("Typical (a question every ~2 min)", 120),
                            ("Casual (a question every ~5 min)", 300)))
    return (f"<h3>What that means for people <i>online</i> (illustration, not a measurement)</h3>"
            f"<p>People spend most of their time reading and thinking, not waiting. By Little’s law, people online ≈ arrival rate × (thinking time + wait). "
            f"With the {label} rate of {lam:g} questions/s ({lam * 60:.0f}/min) and a typical {ms_s(ans)} wait:</p>"
            "<table><thead><tr><th>Behaviour (assumed)</th><th class='num'>Time between asks</th>"
            f"<th class='num'>People online the demo could carry</th></tr></thead><tbody>{rows_}</tbody></table>")


def h2_section(charts: dict, f: dict) -> str:
    lam, fail = f["kLam"], f["fail_lam"]
    words = (f"For a 90-second window the demo kept up with a steady stream of about <b>{lam * 60:.0f} fresh questions per minute</b> ({lam:g}/s)"
             + (f"; at {fail:g}/s ({fail * 60:.0f}/min) the promise broke." if fail
                else f"; even our top rate of {f['h2_max']:g}/s ({f['h2_max'] * 60:.0f}/min) did not break it, so the true limit is higher than we tested for that short window (the sustained test below shows the longer-term limit).")
             if lam else "No tested arrival rate met the 90% line.")
    return f"""<h2>5. Result 2 — questions arriving on their own schedule (H2)</h2>
<div class="callout"><b>In one sentence.</b> {words}</div>
<p>In real life nobody waits politely for the person before them. Here new questions arrive at a fixed rate for 90 seconds per level.
If the server is slower than the arrivals, the backlog grows and every later answer is later still.</p>
{fig(charts, 'h2_open_loop', "<b>Figure 9 — Arrivals vs delivery.</b> Left: share answered in time at each arrival rate. Right: the dashed line is what we offered; the teal line is what came back as good answers. When teal bends away from dashed, the server has hit its ceiling.")}
{fig(charts, 'h2_latency', "<b>Figure 10 — The wait grows before it collapses.</b> The shaded band runs from the typical answer to the slowest tenth.")}
{_h2_table(f)}{_users_table(f)}"""


def h3_section(charts: dict, f: dict) -> str:
    h3, h1 = f["h3"], f["h1"]
    faq = h3[-1] if h3 else None
    ref = h1[0]["total_ms_p50"] if h1 else None
    words = (f"When {esc(faq['concurrency'])} people asked the same three FAQs, <b>{pct(faq['answer_cache_hit_rate'])}</b> were served from the answer cache "
             f"with a typical wait of {ms_s(faq['total_ms_p50'])}" + (f", versus {ms_s(ref)} for a fresh question on a quiet server" if ref else "") + "."
             if faq else "FAQ card not run.")
    return f"""<h2>6. Result 3 — the same questions again and again (H3)</h2>
<div class="callout info"><b>In one sentence.</b> {words}</div>
{fig(charts, 'h3_faq', "<b>Figure 11 — Cache benefit.</b> Repeated questions skip the LLM, so they come back quickly and cost nothing extra. The amber line is a fresh question when the server is quiet, for comparison.")}
<p class="muted">Note: even cache hits show multi-second waits at high concurrency. That is queueing in front of the small VM, not LLM time, which is why the FAQ figures are reported separately and never counted as fresh-question capacity.</p>"""


def h4_section(charts: dict, f: dict) -> str:
    s = f["soak"]
    if not s:
        return ""
    words = (f"Holding <b>{esc(s['concurrency'])} people</b> asking non-stop for {s['duration_s'] / 60:.0f} minutes produced {esc(s['n'])} answers, "
             f"{pct(s['attainment'])} on time. Typical wait moved from {s['first']:.1f} s (first third) to {s['last']:.1f} s (last third), "
             f"a drift of {s['drift_pct']:+.0f}% — "
             + ("stable, no sign of a slow leak or overheating." if s["stable"]
                else "so the crowd size that looked fine for two minutes <b>could not be held</b>."))
    why = "" if s["stable"] else ("<p>This is the most important surprise of the test. The short sweeps above are honest, but they are short. "
        "When the same crowd stays for several minutes, the server slows down — the explanation is in the next section.</p>")
    return f"""<h2>7. Result 4 — does it hold up over time? (H4 soak)</h2>
<div class="callout {'' if s['stable'] else 'warn'}"><b>In one sentence.</b> {words}</div>{why}
{fig(charts, 'h4_soak', "<b>Figure 12 — Every answer during the soak.</b> A flat rolling average means steady behaviour. A rising line means something (CPU allowance, memory, rate limits) was wearing out.")}"""


def _h5_table(sus: dict) -> str:
    def cell(st, key, fmt):
        return f"<td class='num'>{fmt(st[key])}</td>" if st else "<td class='num'>—</td>"

    body = ""
    for l in sus["levels"]:
        st, w = l["steady"], l["whole"]
        ok = l["sustainable"]
        body += (f"<tr class='{'ok' if ok else 'bad'}'><td>{esc(l['label'])}</td>"
                 f"<td class='num'>{pct(w['attainment']) if w else '—'}</td>"
                 f"{cell(st, 'attainment', pct)}{cell(st, 'total_ms_p50', ms_s)}{cell(st, 'total_ms_p90', ms_s)}"
                 f"<td>{'✔ sustainable' if ok else '✖ not sustainable'}</td></tr>")
    return ("<table><thead><tr><th>Load</th><th class='num'>On time, whole run</th><th class='num'>On time, after burst</th>"
            "<th class='num'>Typical wait, after burst</th><th class='num'>Slowest 10% from</th><th>Verdict</th></tr></thead>"
            f"<tbody>{body}</tbody></table>")


def h5_section(charts: dict, f: dict) -> str:
    sus = f["sustain"]
    if not sus:
        return ""
    c, o = sus["best_closed"], sus["best_open"]
    words = (("A non-stop crowd of <b>%s people</b>" % c if c else f"No non-stop crowd of {sus['min_closed_tested']} people or more")
             + " and "
             + (f"a steady stream of <b>{o * 60:.0f} questions a minute</b>" if o else "no steady stream we tried")
             + f" held up once the burst allowance was used up ({sus['soak_s'] / 60:.1f} minutes per level, judged after the first 2 minutes).")
    return f"""<h2>8. Result 5 — what the server can sustain (H5)</h2>
<div class="callout"><b>In one sentence.</b> {words}</div>
<h3>Burst vs sustained — the idea in plain English</h3>
<p>The tested machine is a <b>shared-core</b> Google Cloud type (<span class="mono">{esc(f['vm_type'])}</span>). Google’s documentation says these machines
are guaranteed only <b>one vCPU’s worth of CPU time in total</b>, and may borrow up to <b>two full vCPUs for roughly two minutes</b> when they have saved up credit while idle.
Think of a phone plan with a fast-data allowance: everything is quick until the allowance runs out, then you are throttled.</p>
<p>So a load that looks fine in a two-minute sweep can be impossible to keep up. To measure the real, all-day capacity we (1) let the server <b>rest</b> for {sus['rest_s']} s so the allowance refills,
(2) apply one steady load for {sus['soak_s']} s, and (3) <b>ignore the first 120 s</b> — the burst — and judge only what happens afterwards.</p>
{fig(charts, 'sustain_ladder', f"<b>Figure 13 — {len(sus['levels'])} sustained tests.</b> Each dot is one answer (height = seconds). The blue-tinted strip is the ~2-minute burst window; the red dashed line is the 25-second promise. Teal panels stay under the line once the burst is over; amber panels drift above it.")}
{_h5_table(sus)}
<p class="muted">Source for the burst rule: Google Cloud Compute Engine documentation, “E2 shared-core machine types”
(<span class="mono">cloud.google.com/compute/docs/general-purpose-machines#e2_shared-core</span>).</p>"""


def health_section(charts: dict, f: dict) -> str:
    cls, sentence = f["server_verdict"]
    p = f["peaks"]
    facts = (f"<p class='muted'>Peaks during the sweep — VM CPU {p['cpu_busy_max']:.0f}% (95th percentile {p['cpu_busy_p95']:.0f}%), "
             f"steal {p['steal_max']:.0f}%, API container {p['api_cpu_max']:.0f}% / {p['api_mem_max']:.0f} MB, "
             f"Postgres {p['pg_cpu_max']:.0f}% / {p['pg_mem_max']:.0f} MB; {p['samples']} samples. "
             "Container percentages come from Docker and are relative to <i>one</i> vCPU, so on this two-vCPU machine a single container can read above 100% "
             "(and, on a shared core, occasionally above 200%). Treat the top panel (whole-VM CPU) as the reliable saturation signal.</p>") if p else ""
    sus = fig(charts, "sustain_telemetry", "<b>Figure 15 — VM vitals during the sustained ladder.</b> CPU is near-idle during the rests, jumps when a level starts, and shows what the VM can keep up once the burst allowance is gone.") if "sustain_telemetry" in charts else ""
    n = 9 if f["sustain"] else 8
    return f"""<h2>{n}. What the server was doing meanwhile</h2>
<div class="callout {cls}"><b>Verdict.</b> {esc(sentence)}</div>
{fig(charts, 'server_telemetry', "<b>Figure 14 — VM vitals during the main sweep.</b> Top: overall CPU use and CPU “stolen” by other tenants on the shared physical core. Middle: which container used the CPU. Bottom: memory against the 4 GB limit. Shaded bands match the test phases in Figure 4.")}{facts}{sus}"""


def _advice(f: dict) -> str:
    bn = f["bottleneck"]["who"]
    sus = f["sustain"]
    items = []
    if bn == "vm" or f["server_verdict"][0] == "warn":
        items += [f"<b>Give the server more CPU.</b> Move from the shared-core <span class='mono'>{esc(f['vm_type'])}</span> to a dedicated-core type "
                  "(for example <span class='mono'>e2-standard-4</span> or a compute-optimised type). Dedicated cores have no burst allowance to run out of.",
                  "<b>Separate the database.</b> Postgres (which holds the vectors and the knowledge graph) and the API compete for the same two vCPUs; give Postgres its own machine or a managed service.",
                  "<b>Make the lookup cheaper.</b> The “look up” step is the one that grows with the crowd — examine its queries (graph hops, number of vectors compared) before buying hardware.",
                  "Re-run this exact protocol afterwards and compare the sustained ceiling (Figure 13) with today’s."]
    elif bn == "llm":
        items += ["<b>A bigger VM alone will not help much.</b> Most of each answer is spent waiting on the external LLM.",
                  "Raise the provider’s concurrency limits, use a faster model tier, cap answer length."]
    else:
        items += ["Both the VM and the LLM contribute; improve them together and re-measure."]
    items.append("Keep the answer cache on and warm it for common questions (FAQ answers cost almost nothing).")
    return "".join(f"<li>{a}</li>" for a in items)


def conclusions_section(f: dict) -> str:
    n = 10 if f["sustain"] else 9
    sus = f["sustain"]
    safe = [f"“On the shared demo ({esc(f['vm_type'])}), for a short burst up to {esc(f['kC'])} people asking fresh questions at once got 9-in-10 answers within 25 s.”"]
    if sus:
        c, o = sus["best_closed"], sus["best_open"]
        safe.append("“Sustained for several minutes, it held "
                    + (f"{c} people asking non-stop" if c else f"fewer than {sus['min_closed_tested']} people asking non-stop")
                    + (f" and about {o * 60:.0f} fresh questions a minute" if o else " and no steady stream we tried") + ".”")
    safe.append("“Repeated FAQ questions are served from the cache almost regardless of the crowd.”")
    li = "".join(f"<li>{x}</li>" for x in safe)
    return f"""<h2>{n}. What you can and cannot say</h2>
<div class="two"><div class="callout"><b>Safe to say</b><ul>{li}</ul></div>
<div class="callout warn"><b>Do not say</b><ul>
<li>“EdgeQuake supports N users” without naming the server, LLM, workspace, duration and date.</li>
<li>That the <i>burst</i> number is the everyday capacity — on this shared-core VM it is not.</li>
<li>That answer <i>quality</i> was tested — it was not.</li>
<li>That production hardware would behave the same.</li></ul></div></div>
<h3>What would raise the ceiling</h3><ul>{_advice(f)}</ul>"""


GLOSSARY = [
    ("Burst vs sustained", "Burst: what a shared-core VM can do for its first ~2 minutes using saved-up CPU credit. Sustained: what it can do indefinitely."),
    ("Knee", "The last load level that still meets the promise; just past it, waiting time climbs fast."),
    ("On time / attainment", "The share of answers that arrived successfully within 25 seconds."),
    ("Typical wait (median, p50)", "Half of answers were faster than this, half slower."),
    ("Slowest 10% (p90)", "Nine in ten answers were faster than this number."),
    ("Closed loop", "A fixed crowd; each person asks again after receiving an answer."),
    ("Open loop", "Questions arrive at a steady rate regardless of whether earlier ones are done."),
    ("Cache hit", "The server reused a stored answer instead of asking the LLM again."),
    ("Goodput", "Good answers delivered per second — throughput that actually counts."),
    ("CPU steal", "Time the VM wanted to run but the shared physical core was busy with another tenant."),
    ("Token", "A word-piece; LLM work and cost are measured in tokens."),
]


def appendix_section(summary: dict, run_id: str, n: int = 10) -> str:
    gl = "".join(f"<dt>{esc(a)}</dt><dd>{esc(b)}</dd>" for a, b in GLOSSARY)
    return f"""<h2>{n}. Reproduce it, and glossary</h2>
<p>Everything is in the repository under <span class="mono">specs/153-workload-benchmark/</span>. Protocol: <span class="mono">07-highload-protocol.md</span>.
This run’s raw evidence: <span class="mono">measurements/{esc(run_id)}/</span> — <span class="mono">provider/units.jsonl</span> and <span class="mono">units-sustain.jsonl</span> (every request),
<span class="mono">machines.json</span> and <span class="mono">gcp-server.json</span> (hardware), <span class="mono">telemetry.jsonl</span> (VM samples), <span class="mono">charts/</span> (300-DPI PNG + vector SVG).</p>
<pre class="mono" style="background:#f1f5f9;padding:8px;border-radius:6px;">make bench153-provider-highload   # BENCH153_RUN_DIR=… BENCH153_API_KEY_FILE=…
BENCH153_PROFILE=sustain make bench153-provider-highload   # sustained ladder (H5)
make bench153-highload-report</pre>
<h3>Glossary</h3><dl class="gloss">{gl}</dl>
<p class="muted">Generated by bench153 · charts by matplotlib (vector SVG embedded) · profile: {esc(summary.get('profile'))} · total run {summary.get('wall_s', 0) / 60:.0f} min.</p>"""
