"""Front half of the report: hero, headline, the server, the protocol in plain English."""

from __future__ import annotations

from bench153.report_util import esc, fig, ms_s, pct, rows


def _sustained_kpi(f: dict) -> str:
    sus = f["sustain"]
    if not sus:
        lam = f["kLam"]
        val = f"{lam * 60:.0f}/min" if lam else "—"
        hint = "fresh questions per minute absorbed in a 90-second window" if lam else "no tested rate met the target"
        return f'<div class="kpi amber"><div class="label">Steady arrival rate</div><div class="value">{val}</div><div class="hint">{hint}</div></div>'
    c, o = sus["best_closed"], sus["best_open"]
    val = f"{c} people" if c else (f"< {sus['min_closed_tested']} people" if sus["min_closed_tested"] else "—")
    tail = f"; steady stream up to {o * 60:.0f}/min" if o else "; no tested steady stream held"
    return (f'<div class="kpi amber"><div class="label">Sustained for minutes</div><div class="value">{val}</div>'
            f'<div class="hint">after the ~2-minute burst allowance{tail}</div></div>')


def _kpis(f: dict) -> str:
    kl1, h3, soak = f["kl1"], f["h3"], f["soak"]
    faq = h3[-1] if h3 else None
    return f"""<div class="kpis">
  <div class="kpi"><div class="label">Short burst (~2 min)</div>
    <div class="value">{esc(f['kC'] if f['kC'] is not None else '—')} people</div>
    <div class="hint">fresh questions, ≥90% answered in 25 s{f" (typical {ms_s(kl1['total_ms_p50'])})" if kl1 else ""}</div></div>
  {_sustained_kpi(f)}
  <div class="kpi sky"><div class="label">Repeated FAQ</div>
    <div class="value">{pct(faq['answer_cache_hit_rate']) if faq else '—'}</div>
    <div class="hint">answered from cache up to {esc(faq['concurrency']) if faq else '—'} at once</div></div>
  <div class="kpi purple"><div class="label">{esc(f"{soak['duration_s'] / 60:.0f}-minute non-stop test") if soak else "Soak"}</div>
    <div class="value">{"Stable" if soak and soak['stable'] else ("Degraded" if soak else "—")}</div>
    <div class="hint">{f"{pct(soak['attainment'])} on time at the burst-crowd size" if soak else "not run"}</div></div>
</div>"""


def _headline(f: dict) -> str:
    burst = (f"For a short burst the demo handled up to <b>{esc(f['kC'])} people</b> asking fresh questions at once"
             + (f" (it first missed at {esc(f['fail_C'])})." if f["fail_C"] else "."))
    sus = f["sustain"]
    if sus:
        c, o = sus["best_closed"], sus["best_open"]
        parts = []
        parts.append(f"a non-stop crowd of <b>{c}</b>" if c else f"no non-stop crowd of {sus['min_closed_tested']} or more")
        parts.append(f"a steady stream of <b>{o * 60:.0f} questions a minute</b>" if o else "no steady stream we tried")
        held = f" Kept up for {sus['soak_s'] / 60:.1f} minutes on end, it sustained " + " and ".join(parts) + "."
    else:
        held = ""
    return burst + held


def _glance(f: dict) -> str:
    """One-table summary of every card, so page 1 stands on its own."""
    sus, soak, h3 = f["sustain"], f["soak"], f["h3"]
    faq = h3[-1] if h3 else None
    lam = f["kLam"]
    rows_ = [("H1", "How many people can ask fresh questions at the same moment (short test)?",
              f"up to <b>{esc(f['kC'])}</b> people" + (f"; first miss at {esc(f['fail_C'])}" if f["fail_C"] else ""), True),
             ("H2", "How fast can fresh questions arrive on their own schedule (90 s per rate)?",
              (f"<b>{'at least ' if f['h2_not_reached'] else ''}{lam * 60:.0f}/min</b>" if lam else "none met the target"), bool(lam))]
    if faq:
        rows_.append(("H3", "What if everyone asks the same FAQ?",
                      f"<b>{pct(faq['answer_cache_hit_rate'])}</b> from cache, up to {esc(faq['concurrency'])} at once", True))
    if soak:
        rows_.append(("H4", f"Can the burst-level crowd ({esc(soak['concurrency'])}) be held for {soak['duration_s'] / 60:.0f} minutes?",
                      f"<b>{'Yes' if soak['stable'] else 'No'}</b> — {pct(soak['attainment'])} on time", bool(soak["stable"])))
    if sus:
        c, o = sus["best_closed"], sus["best_open"]
        rows_.append(("H5", "What can it sustain once the burst allowance is spent?",
                      (f"<b>{c}</b> people" if c else "no crowd tested") + " · " + (f"<b>{o * 60:.0f}/min</b>" if o else "no stream tested"), True))
    body = "".join(f"<tr class='{'ok' if ok else 'bad'}'><td><b>{k}</b></td><td>{q}</td><td>{a}</td></tr>" for k, q, a, ok in rows_)
    return ("<h3>Results at a glance</h3><table class='glance'><thead><tr><th>Card</th><th>Question</th><th>Answer</th></tr></thead>"
            f"<tbody>{body}</tbody></table>"
            "<p class='muted'>“On time” always means: answered successfully within 25 seconds. A level passes when at least 90% of its answers are on time.</p>")


def hero(summary: dict, f: dict, run_id: str, vm: dict) -> str:
    over = _headline(f)
    return f"""<section class="hero">
  <div class="eyebrow">SPEC-153 · higher concurrent-user workload · pushed run</div>
  <h1>How many people can ask EdgeQuake at once?</h1>
  <p>We pointed a load generator at the shared demo and kept adding people until answers got too slow. Everything below is measured,
  on one named Google Cloud server, and explained in plain English.</p>
  <div class="meta">
    <div><strong>Run</strong>{esc(run_id)}</div>
    <div><strong>Target</strong>{esc(summary.get('base_url'))}</div>
    <div><strong>Product version</strong>{esc(summary.get('health_version'))}</div>
    <div><strong>Server</strong>{esc(vm.get('instance'))} · {esc(vm.get('machine_type'))}</div>
    <div><strong>Tenant</strong><span class="mono">{esc(summary.get('tenant_id'))}</span></div>
    <div><strong>Workspace</strong><span class="mono">{esc(summary.get('workspace_id'))}</span></div>
  </div></section>
{_kpis(f)}
<div class="callout"><b>The short answer.</b> {over} {esc(f['server_verdict'][1])}</div>
{_glance(f)}"""


def question_section() -> str:
    return """<h2>1. The question, in plain English</h2>
<p>A demo can feel fast when one person uses it and slow when a whole team does. We wanted to know <b>where that change happens</b>:
how many people can ask <i>brand-new</i> questions at the same moment and still get an answer in about <b>25 seconds</b>,
most of the time (at least <b>9 answers in 10</b>).</p>
<p>The point where it stops meeting that promise is called the <b>knee</b>. Below the knee, adding people costs little.
Past it, everyone waits longer and fewer good answers come out per second.</p>"""


def _kv(d: dict, keys: list[tuple[str, str]]) -> list[tuple[str, str]]:
    return [(lbl, esc(d.get(k))) for lbl, k in keys if d.get(k) not in (None, "")]


def server_section(charts: dict, machines: dict, dns: dict) -> str:
    sut, lg = machines.get("sut") or {}, machines.get("load_generator") or {}
    gcp = sut.get("gcp") or {}
    vm, host = gcp.get("vm") or {}, gcp.get("host") or {}
    if not vm:
        return "<h2>2. The server that was tested</h2><p class='muted'>GCP inventory unavailable for this run.</p>"
    disks = "<br/>".join(f"{esc(d['name'])} — {esc(d['size_gb'])} GB {esc(d.get('type'))}"
                         f"{' (boot)' if d.get('boot') else ' (data)'}" for d in vm.get("disks", []))
    ct = "<br/>".join(f"<span class='mono'>{esc(c['name'])}</span> — {esc(c['image'])}" for c in host.get("containers", []))
    pg = host.get("postgres") or {}
    dns_txt = (f"<span class='mono'>{esc(dns.get('host'))}</span> → <span class='mono'>{esc(dns.get('ip'))}</span> "
               f"({'matches' if dns.get('match') else 'does NOT match'} the VM’s public IP)")
    hw = rows([
        ("Provider / project", f"Google Cloud · project <span class='mono'>{esc(vm.get('project'))}</span>"),
        ("Instance", f"<b>{esc(vm.get('instance'))}</b> (id <span class='mono'>{esc(vm.get('instance_id'))}</span>)"),
        ("Zone", esc(vm.get("zone"))),
        ("Machine type", f"<b>{esc(vm.get('machine_type'))}</b> — {esc(vm.get('machine_description'))}"),
        ("CPU", f"{esc(host.get('cpu', vm.get('cpu_platform')))} ({esc(vm.get('cpu_platform'))}) · "
                f"{esc(vm.get('vcpus'))} vCPUs, shared-core = {esc(vm.get('shared_core'))}"),
        ("Memory", f"{(vm.get('memory_mb') or 0) / 1024:.0f} GB (OS reports {esc(host.get('mem_mb'))} MB)"),
        ("Disks", disks),
        ("Operating system", f"{esc(host.get('os'))} · kernel {esc(host.get('kernel'))}"),
        ("Network", f"VPC {esc(vm.get('vpc'))} / {esc(vm.get('subnet'))} · internal {esc(vm.get('internal_ip'))} · "
                    f"public {esc(vm.get('external_ip'))}"),
        ("DNS check", dns_txt),
        ("Scaling", "Single VM, no autoscaling, no load balancer. Standard (non-preemptible), created " + esc(vm.get("created", "")[:10])),
    ])
    sw = rows([
        ("Containers on the VM", ct),
        ("EdgeQuake release", f"v{esc(sut.get('product_version'))}"),
        ("Database", f"Postgres with Apache AGE (graph) + pgvector (embeddings) · max_connections {esc(pg.get('max_connections'))}, "
                     f"shared_buffers {esc(pg.get('shared_buffers'))}, work_mem {esc(pg.get('work_mem'))}"),
        ("Answer model", esc(", ".join((sut.get("workspace_answer_llm") or {}).get("models") or ["?"])) + " via " +
         esc(", ".join((sut.get("workspace_answer_llm") or {}).get("providers") or ["?"])) + " (external API)"),
        ("TLS / front door", "Caddy reverse proxy terminates HTTPS; port 80 redirects to 443"),
        ("Authentication", "X-API-Key header (value never stored in this report)"),
    ])
    client = rows([
        ("Machine", f"{esc(lg.get('hostname'))}"),
        ("Hardware", f"{esc(lg.get('cpu_model'))} · {esc(lg.get('cpu_logical'))} logical cores · {esc(lg.get('mem_gb'))} GB"),
        ("Software", f"{esc(((lg.get('os') or {}).get('sw_vers') or {}).get('product_name'))} "
                     f"{esc(((lg.get('os') or {}).get('sw_vers') or {}).get('product_version'))} · Python {esc(lg.get('python'))}"),
        ("Network path", "the client’s own internet connection → public internet → Google Cloud us-central1"),
    ])
    return f"""<h2 class="pb">2. The server that was tested</h2>
<p>The public demo at <b>demo.edgequake.com</b> is <b>not</b> a serverless service and not a large cluster. It is <b>one small Google Cloud virtual machine</b>
running four containers. The exact facts below were read from Google Cloud and from the VM itself at the start of the run.</p>
{fig(charts, 'architecture', "<b>Figure 1 — The moving parts.</b> Questions travel from the load generator over the internet to the VM. On the VM, Caddy passes them to the EdgeQuake API, which looks things up in Postgres and calls Mistral’s hosted LLM to understand the question and write the answer.")}
<h3>The virtual machine (system under test)</h3><table>{hw}</table>
<h3>Software and configuration on that VM</h3><table>{sw}</table>
<div class="callout info"><b>Why this matters.</b> An <span class="mono">e2-medium</span> gives two virtual CPUs but only one vCPU’s worth of CPU time in total (with short bursts above that), and 4 GB of memory that Postgres,
the API, the web UI and Caddy all share. It is a cost-optimised demo box. Numbers here describe <b>this box and this LLM account</b>, not what EdgeQuake can do on production hardware.
A Cloud Run service also named <span class='mono'>edgequake-api</span> exists in the same project but is <b>not</b> what serves the demo.</div>
<h3>The load generator (the machine that fired the questions)</h3><table>{client}</table>
<p class="muted">The load generator’s power is listed for honesty only. It sends small HTTPS requests, so it is not what limits the result.</p>"""


def protocol_section(charts: dict, summary: dict) -> str:
    cfg_h1 = ", ".join(str(l["concurrency"]) for l in (summary["H1_cold_closed"].get("levels") or []))
    cfg_h2 = ", ".join(f"{l['lambda_qps']:g}" for l in (summary["H2_open_loop"].get("levels") or []))
    return f"""<h2>3. How the test works — in plain English</h2>
<p>Think of the demo as a coffee counter. One barista (the server) takes an order, phones a specialist supplier (the LLM), and hands the drink over.
We want to know how big the queue can get before customers wait too long.</p>
{fig(charts, 'ask_pipeline', "<b>Figure 2 — One question, five steps.</b> Amber steps call Mistral’s hosted LLM over the internet; teal steps run on our VM. The small grey labels are the exact timing fields the API returns, which we use to show where time goes.")}
<h3>The recipe</h3>
<ol class="steps">
  <li><b>Check the counter is open.</b> We read <span class="mono">/health</span> and <span class="mono">/ready</span> and send one real, authenticated question. If the server is unhealthy or rejects our key, the run stops (BLOCKED) instead of measuring garbage.</li>
  <li><b>Use real, never-repeated questions.</b> Questions are about the papers already uploaded to the demo workspace. Every “fresh” question carries a one-off tag, so the server cannot reuse a stored answer. This measures real work, not memory.</li>
  <li><b>Turn the dial two different ways</b> (Figure 3): <b>H1</b> — a fixed crowd of {esc(cfg_h1)} people each asking again as soon as they get an answer; <b>H2</b> — new questions arriving at a steady rate of {esc(cfg_h2)} per second, whether or not earlier ones have finished.</li>
  <li><b>Score every answer.</b> An answer is <b>good</b> only if it came back successfully, was not empty, and took <b>under 25 seconds</b> (and, for fresh questions, was not a cache hit). A load level <b>passes</b> if at least 90% of its answers are good, fewer than 5% errors occur and fewer than 3 “too many requests” replies are seen.</li>
  <li><b>Find the knee.</b> The knee is the last level that passes before the first one that fails. Because you asked us to push further, we <b>kept going past the knee</b> to watch how the service degrades, with safety brakes: stop if over 25% of requests error, 10 or more “too many requests” replies appear, or under 10% of answers are good.</li>
  <li><b>Test the easy case separately (H3).</b> The same three FAQ questions are asked over and over by up to 32 people. Those are answered from a cache, so they are reported on their own and never mixed into the fresh-question capacity.</li>
  <li><b>Check it holds up (H4).</b> We hold the comfortable crowd steady for several minutes to see whether answers slowly get slower. This turned out to matter: a small shared-core VM is allowed to run flat-out only for about two minutes (see “burst vs sustained” below).</li>
  <li><b>Find the <i>sustained</i> ceiling (H5).</b> We rest the server for a few minutes so its burst allowance refills, apply one steady load for 3½ minutes, and judge only the part <i>after</i> the first two minutes. A load that still passes there is something the server can do all day.</li>
  <li><b>Watch the server while it happens.</b> Every few seconds we read the VM’s CPU, memory and per-container usage (read-only) to see whether the hardware or the LLM was the limit.</li>
</ol>
{fig(charts, 'loop_models', "<b>Figure 3 — Two ways to apply load.</b> Left: a fixed crowd (closed loop) always keeps the same number of questions in flight. Right: a steady stream of arrivals (open loop) keeps coming even when the server is behind, so delays pile up. Real traffic looks more like the right-hand picture.")}
{fig(charts, 'phase_timeline', "<b>Figure 4 — What actually ran, and when.</b> Each block is one load level. Faded blocks missed the 90% success line.")}
<div class="two">
<div class="callout"><b>Rules of fair play</b><ul>
  <li>One tenant and one workspace (IDs in the header), same model throughout.</li>
  <li>Fresh questions never repeat; FAQ tests are reported separately.</li>
  <li>Never more than 32 people at once, and never faster than 0.6 questions/s.</li>
  <li>Every request and its timings are saved (<span class="mono">units.jsonl</span>).</li></ul></div>
<div class="callout warn"><b>What we did not do</b><ul>
  <li>No document uploads, deletes or OCR on the shared demo.</li>
  <li>We do not judge answer <i>quality</i> (that is a different benchmark).</li>
  <li>We do not claim this is “the” capacity of EdgeQuake — only of this VM and this LLM account, on this day.</li></ul></div>
</div>"""
