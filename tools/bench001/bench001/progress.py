"""Run progress + history ledger for SPEC-001 smoke→core ladder."""

from __future__ import annotations

import json
import shutil
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .paths import ARTIFACTS_DIR, stage_artifact_dir

# Canonical Acc pipeline phases (order used in LIVE.md board).
PIPELINE_PHASES: tuple[str, ...] = (
    "prepare",
    "ingest_eq",
    "query_parallel",
    "score_parallel",
    "report",
)

_lock = threading.Lock()
_run_started_mono: float | None = None
_run_started_utc: str | None = None
_active_stage: str | None = None


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def format_duration(seconds: float | None) -> str:
    """Human-readable duration for logs (e.g. ``3m12s``, ``1h05m03s``)."""
    if seconds is None:
        return "—"
    try:
        s = int(round(float(seconds)))
    except (TypeError, ValueError):
        return "—"
    if s < 0:
        s = 0
    h, rem = divmod(s, 3600)
    m, sec = divmod(rem, 60)
    if h:
        return f"{h}h{m:02d}m{sec:02d}s"
    if m:
        return f"{m}m{sec:02d}s"
    return f"{sec}s"


def eta_seconds(done: int | float, total: int | float, elapsed_s: float) -> float | None:
    """Linear ETA from completed units. Returns None until progress is informative."""
    try:
        d = float(done)
        t = float(total)
        e = float(elapsed_s)
    except (TypeError, ValueError):
        return None
    if d <= 0 or t <= 0 or e <= 0:
        return None
    if d >= t:
        return 0.0
    # Need a few samples before trusting ETA (avoid wild early spikes).
    if d < 1 and t > 1:
        return None
    rate = d / e
    if rate <= 0:
        return None
    return (t - d) / rate


def rate_per_min(done: int | float, elapsed_s: float) -> float | None:
    try:
        d = float(done)
        e = float(elapsed_s)
    except (TypeError, ValueError):
        return None
    if d <= 0 or e <= 0:
        return None
    return d / e * 60.0


def progress_path(stage: str) -> Path:
    return stage_artifact_dir(stage) / "progress.json"


def live_path(stage: str) -> Path:
    return stage_artifact_dir(stage) / "LIVE.md"


def live_root_path() -> Path:
    ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)
    return ARTIFACTS_DIR / "LIVE.md"


def progress_jsonl_path(stage: str) -> Path:
    return stage_artifact_dir(stage) / "logs" / "progress.jsonl"


def history_root() -> Path:
    d = ARTIFACTS_DIR / "history"
    d.mkdir(parents=True, exist_ok=True)
    return d


def progress_md_path() -> Path:
    return ARTIFACTS_DIR / "PROGRESS.md"


def begin_run(stage: str, *, detail: str | None = None) -> None:
    """Reset live run clock and write an initial LIVE.md board."""
    global _run_started_mono, _run_started_utc, _active_stage
    with _lock:
        _run_started_mono = time.monotonic()
        _run_started_utc = utc_now()
        _active_stage = stage
    data = {
        "stage": stage,
        "started_at_utc": _run_started_utc,
        "updated_at_utc": _run_started_utc,
        "phases": [],
        "current_phase": "prepare",
        "counts": {},
        "run_elapsed_s": 0.0,
    }
    save_progress(stage, data)
    _append_jsonl(
        stage,
        {
            "at_utc": _run_started_utc,
            "event": "begin_run",
            "stage": stage,
            "detail": detail,
        },
    )
    write_live_md(stage, data)
    msg = f"[bench001:{stage}] RUN START"
    if detail:
        msg += f" — {detail}"
    print(msg, flush=True)
    print(f"  live board → {live_path(stage)} (also {live_root_path()})", flush=True)


def run_elapsed_s() -> float:
    if _run_started_mono is None:
        return 0.0
    return max(0.0, time.monotonic() - _run_started_mono)


def load_progress(stage: str) -> dict[str, Any]:
    path = progress_path(stage)
    if path.exists():
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            pass
    return {
        "stage": stage,
        "started_at_utc": utc_now(),
        "updated_at_utc": utc_now(),
        "phases": [],
        "current_phase": None,
        "counts": {},
    }


def save_progress(stage: str, data: dict[str, Any]) -> Path:
    data["updated_at_utc"] = utc_now()
    data["run_elapsed_s"] = run_elapsed_s()
    path = progress_path(stage)
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")
    return path


def _append_jsonl(stage: str, row: dict[str, Any]) -> None:
    path = progress_jsonl_path(stage)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as fh:
        fh.write(json.dumps(row, ensure_ascii=False) + "\n")


def mark_phase(
    stage: str,
    phase: str,
    *,
    status: str = "running",
    detail: str | None = None,
    counts: dict[str, Any] | None = None,
    done: int | float | None = None,
    total: int | float | None = None,
    phase_elapsed_s: float | None = None,
    quiet: bool = False,
) -> dict[str, Any]:
    """Append/update a phase tick, refresh LIVE.md, print a progress banner."""
    data = load_progress(stage)
    data["current_phase"] = phase
    tick: dict[str, Any] = {
        "phase": phase,
        "status": status,
        "at_utc": utc_now(),
        "run_elapsed_s": run_elapsed_s(),
    }
    if detail:
        tick["detail"] = detail
    if counts:
        tick["counts"] = counts
        data["counts"] = {**(data.get("counts") or {}), **counts}
    if done is not None:
        tick["done"] = done
        data["counts"] = {**(data.get("counts") or {}), "done": done}
    if total is not None:
        tick["total"] = total
        data["counts"] = {**(data.get("counts") or {}), "total": total}
    if phase_elapsed_s is not None:
        tick["phase_elapsed_s"] = phase_elapsed_s

    eta: float | None = None
    if done is not None and total is not None and phase_elapsed_s is not None:
        eta = eta_seconds(done, total, phase_elapsed_s)
    elif counts:
        # Common ingest/query keys.
        d = counts.get("done")
        t = counts.get("total")
        e = counts.get("elapsed_s") or counts.get("ingest_elapsed_s") or phase_elapsed_s
        if d is not None and t is not None and e is not None:
            eta = eta_seconds(d, t, float(e))
        elif counts.get("ingest_pct") is not None and counts.get("ingest_elapsed_s") is not None:
            try:
                pct = float(counts["ingest_pct"])
                el = float(counts["ingest_elapsed_s"])
                if 0.05 < pct < 0.99 and el > 0:
                    eta = el * (1.0 - pct) / pct
            except (TypeError, ValueError):
                eta = None
        if eta is None and counts.get("ingest_eta_s") is not None:
            try:
                eta = float(counts["ingest_eta_s"])
            except (TypeError, ValueError):
                eta = None
    if eta is not None:
        tick["eta_s"] = eta
        data["counts"] = {**(data.get("counts") or {}), "eta_s": eta}

    phases = list(data.get("phases") or [])
    # Collapse consecutive same-phase running ticks into one update.
    if (
        phases
        and phases[-1].get("phase") == phase
        and phases[-1].get("status") == "running"
        and status == "running"
    ):
        phases[-1] = tick
    else:
        phases.append(tick)
    data["phases"] = phases
    save_progress(stage, data)
    write_live_md(stage, data)
    _append_jsonl(
        stage,
        {
            "at_utc": tick["at_utc"],
            "event": "phase",
            "phase": phase,
            "status": status,
            "detail": detail,
            "done": done,
            "total": total,
            "eta_s": eta,
            "run_elapsed_s": tick["run_elapsed_s"],
            "counts": counts,
        },
    )

    if not quiet:
        parts = [f"[bench001:{stage}] {phase} ({status})"]
        if detail:
            parts.append(f"— {detail}")
        bits: list[str] = []
        if done is not None and total is not None:
            bits.append(f"{done}/{total}")
        if eta is not None:
            bits.append(f"eta={format_duration(eta)}")
        bits.append(f"run={format_duration(tick['run_elapsed_s'])}")
        if phase_elapsed_s is not None:
            bits.append(f"phase={format_duration(phase_elapsed_s)}")
        if bits:
            parts.append("| " + " ".join(bits))
        if counts and done is None:
            # Compact count dump only when not already shown as done/total.
            interesting = {
                k: v
                for k, v in counts.items()
                if k
                in {
                    "ingest_pct",
                    "ingest_eta_s",
                    "ingest_docs_done",
                    "ingest_docs_total",
                    "eq_empty_context_rate",
                    "lr_empty_context_rate",
                    "eq_acc",
                    "lr_acc",
                }
                and v is not None
            }
            if interesting:
                parts.append(str(interesting))
        print(" ".join(parts), flush=True)
    return data


def write_live_md(stage: str, data: dict[str, Any] | None = None) -> Path:
    """Rewrite a human-readable LIVE.md board for ``watch`` / ``tail`` monitoring."""
    data = data or load_progress(stage)
    elapsed = float(data.get("run_elapsed_s") or run_elapsed_s())
    current = data.get("current_phase") or "—"
    counts = data.get("counts") or {}
    phases = list(data.get("phases") or [])

    # Latest status per phase name.
    latest: dict[str, dict[str, Any]] = {}
    for tick in phases:
        name = str(tick.get("phase") or "")
        if name:
            latest[name] = tick

    def _glyph(name: str) -> str:
        tick = latest.get(name)
        if not tick:
            return "○"
        st = str(tick.get("status") or "")
        if st == "done":
            return "✓"
        if st == "failed":
            return "✗"
        if st == "running":
            return "●"
        return "○"

    pipeline = " → ".join(f"{_glyph(p)} {p}" for p in PIPELINE_PHASES)
    # Also show any non-canonical phases seen (query_eq / query_lr).
    extras = [n for n in latest if n not in PIPELINE_PHASES]
    if extras:
        pipeline += "  |  " + " ".join(f"{_glyph(n)} {n}" for n in extras)

    cur = latest.get(str(current)) or {}
    eta = cur.get("eta_s")
    if eta is None and counts.get("eta_s") is not None:
        eta = counts.get("eta_s")
    done = cur.get("done", counts.get("done"))
    total = cur.get("total", counts.get("total"))
    detail = cur.get("detail") or "—"

    lines = [
        f"# bench001 LIVE — `{stage}`",
        "",
        f"- **updated:** `{data.get('updated_at_utc') or utc_now()}`",
        f"- **started:** `{data.get('started_at_utc') or _run_started_utc or '—'}`",
        f"- **run elapsed:** `{format_duration(elapsed)}`",
        f"- **phase:** `{current}` ({cur.get('status') or '—'})",
        f"- **progress:** `{done}/{total}`" if done is not None and total is not None else "- **progress:** —",
        f"- **ETA (phase):** `{format_duration(float(eta)) if eta is not None else '—'}`",
        f"- **detail:** {detail}",
        "",
        "## Corpus / chunking",
        "",
        f"- **docs:** `{counts.get('n_docs') or counts.get('ingest_docs_total') or '—'}`"
        f"  (done `{counts.get('ingest_docs_done') or '—'}`)",
        f"- **chunk size / overlap:** `{counts.get('chunk_token_size') or '—'}` / "
        f"`{counts.get('chunk_overlap_token_size') or '—'}`",
        f"- **indexed chunks:** `{counts.get('chunk_count') or '—'}`",
        f"- **corpus chars:** `{counts.get('corpus_chars') or '—'}`"
        f"  capped=`{counts.get('ingest_capped')}`",
        f"- **questions:** `{counts.get('n_questions') or '—'}`",
        "",
        "## Pipeline",
        "",
        pipeline,
        "",
        "## Recent ticks",
        "",
        "| at (UTC) | phase | status | detail | eta | run |",
        "|----------|-------|--------|--------|-----|-----|",
    ]
    for tick in phases[-12:]:
        lines.append(
            "| {at} | {phase} | {status} | {detail} | {eta} | {run} |".format(
                at=tick.get("at_utc") or "—",
                phase=tick.get("phase") or "—",
                status=tick.get("status") or "—",
                detail=str(tick.get("detail") or "—").replace("|", "/")[:80],
                eta=format_duration(tick.get("eta_s")),
                run=format_duration(tick.get("run_elapsed_s")),
            )
        )
    lines.extend(
        [
            "",
            "## Monitor",
            "",
            "```bash",
            f"make bench001-watch STAGE={stage}",
            f"# or:  watch -n 2 cat {live_path(stage)}",
            f"# or:  tail -f {progress_jsonl_path(stage)}",
            "```",
            "",
        ]
    )
    text = "\n".join(lines)
    path = live_path(stage)
    path.write_text(text, encoding="utf-8")
    # Root pointer always shows the active run.
    live_root_path().write_text(text, encoding="utf-8")
    return path


def print_unit_progress(
    label: str,
    done: int,
    total: int,
    *,
    elapsed_s: float,
    extra: str = "",
) -> None:
    """Single-line unit progress with rate + ETA (EQ/LR query, judge samples)."""
    rate = rate_per_min(done, elapsed_s)
    eta = eta_seconds(done, total, elapsed_s)
    rate_s = f" rate={rate:.1f}/min" if rate is not None else ""
    eta_s = f" eta={format_duration(eta)}" if eta is not None else ""
    run_s = f" run={format_duration(run_elapsed_s())}" if _run_started_mono else ""
    extra_s = f" {extra}" if extra else ""
    print(
        f"  {label} progress {done}/{total} "
        f"elapsed={format_duration(elapsed_s)}{rate_s}{eta_s}{run_s}{extra_s}",
        flush=True,
    )


def empty_context_rate(preds: list[dict[str, Any]]) -> float:
    if not preds:
        return 1.0
    empty = 0
    for p in preds:
        ctx = p.get("context") or []
        if isinstance(ctx, str):
            nonempty = bool(ctx.strip())
        else:
            nonempty = any(isinstance(c, str) and c.strip() for c in ctx)
        if not nonempty:
            empty += 1
    return empty / len(preds)


# Thin SSOT — safe to commit (SPEC-097 / LAW-G2).
_ARCHIVE_THIN = (
    "scorecard.json",
    "SUMMARY.md",
    "BUSINESS_REPORT.md",
    "EXEC_SUMMARY.txt",
    "meta.json",
    "eq_workspace.json",
    "progress.json",
    "LIVE.md",
)

# Fat regenerable forensics — local-only, gitignored (SPEC-097 / LAW-G3).
_ARCHIVE_FAT = (
    "eval_eq.json",
    "eval_lr.json",
    "eval_eq.raw.json",
    "eval_lr.raw.json",
    "predictions_eq.json",
    "predictions_lr.json",
)


def _write_local_only_md(dest: Path, *, present_fat: list[str]) -> None:
    """Document which archive files must not be committed (SPEC-097)."""
    lines = [
        "# LOCAL_ONLY — fat bench001 artifacts (SPEC-097 / GH-351)",
        "",
        "These files are written for local forensics and are **gitignored**.",
        "Do not `git add -f` them. Acc claims live in `scorecard.json` /",
        "`SUMMARY.md` / `publish/` peers.",
        "",
        "Regenerate via `make bench001-*`.",
        "",
    ]
    if present_fat:
        lines.append("Present in this archive:")
        lines.append("")
        for name in present_fat:
            lines.append(f"- `{name}`")
        lines.append("")
    else:
        lines.append("_No fat files were present when this archive was written._")
        lines.append("")
    (dest / "LOCAL_ONLY.md").write_text("\n".join(lines), encoding="utf-8")


def archive_run(stage: str, scorecard: dict[str, Any]) -> Path:
    """Copy key artifacts into history/<stage>-<timestamp>/ and refresh PROGRESS.md.

    Thin scorecards/reports stay VCS-eligible; fat predictions/eval/logs are
    copied locally only (SPEC-097 / GH-351).
    """
    ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    dest = history_root() / f"{stage}-{ts}"
    dest.mkdir(parents=True, exist_ok=True)
    src = stage_artifact_dir(stage)
    present_fat: list[str] = []
    for name in _ARCHIVE_THIN + _ARCHIVE_FAT:
        p = src / name
        if p.exists():
            shutil.copy2(p, dest / name)
            if name in _ARCHIVE_FAT:
                present_fat.append(name)
    logs_src = src / "logs" / "progress.jsonl"
    if logs_src.exists():
        (dest / "logs").mkdir(exist_ok=True)
        shutil.copy2(logs_src, dest / "logs" / "progress.jsonl")
        present_fat.append("logs/progress.jsonl")
    _write_local_only_md(dest, present_fat=present_fat)
    write_progress_md(scorecard=scorecard, archive_dir=dest)
    return dest


def write_progress_md(*, scorecard: dict[str, Any] | None = None, archive_dir: Path | None = None) -> Path:
    """Rewrite ladder PROGRESS.md from history/*/scorecard.json (+ optional latest)."""
    rows: list[dict[str, Any]] = []
    for sc_path in sorted(history_root().glob("*/scorecard.json")):
        try:
            sc = json.loads(sc_path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        rows.append(
            {
                "dir": sc_path.parent.name,
                "stage": sc.get("stage"),
                "valid": sc.get("valid"),
                "created_at_utc": sc.get("created_at_utc"),
                "eq": (sc.get("metrics") or {}).get("eq", {}).get("overall_acc"),
                "lr": (sc.get("metrics") or {}).get("lr", {}).get("overall_acc"),
                "delta": (sc.get("metrics") or {})
                .get("delta_eq_minus_lr", {})
                .get("overall_acc"),
                "n": (sc.get("ops") or {}).get("n_questions"),
                "judge": (sc.get("pins") or {}).get("judge"),
                "q_conc": (sc.get("pins") or {}).get("query_concurrency"),
                "e_conc": (sc.get("pins") or {}).get("eval_concurrency"),
                "eq_empty_ctx": (sc.get("ops") or {}).get("eq_empty_context_rate"),
                "lr_empty_ctx": (sc.get("ops") or {}).get("lr_empty_context_rate"),
            }
        )
    if scorecard is not None and archive_dir is not None:
        # Ensure latest is present even if copy raced.
        pass

    lines = [
        "# SPEC-001 progression ladder",
        "",
        "Smoke → core Acc history (newest last). Each row is an archived `scorecard.json`.",
        "",
        "| Run | Stage | Valid | n | EQ Acc | LR Acc | Δ | Judge | Q∥ | Eval∥ | EQ empty-ctx | LR empty-ctx |",
        "|-----|-------|-------|---|--------|--------|---|-------|----|-------|--------------|--------------|",
    ]
    for r in rows:
        lines.append(
            "| {dir} | {stage} | {valid} | {n} | {eq} | {lr} | {delta} | {judge} | {qc} | {ec} | {eqc} | {lrc} |".format(
                dir=r["dir"],
                stage=r.get("stage"),
                valid=r.get("valid"),
                n=r.get("n"),
                eq=_fmt(r.get("eq")),
                lr=_fmt(r.get("lr")),
                delta=_fmt(r.get("delta"), signed=True),
                judge=r.get("judge"),
                qc=r.get("q_conc"),
                ec=r.get("e_conc"),
                eqc=_fmt(r.get("eq_empty_ctx")),
                lrc=_fmt(r.get("lr_empty_ctx")),
            )
        )
    if not rows:
        lines.append("| _(none yet)_ | | | | | | | | | | | |")
    lines.extend(
        [
            "",
            "## Ladder meaning",
            "",
            "1. **dry-run** — harness plumbing (`valid: false`)",
            "2. **smoke** — 40 stratified medical IDs, dual-SUT official judge",
            "3. **core** — cost-gated full medical+novel fixture",
            "",
            "Compare runs: `python3 -m bench001.cli report smoke --compare history/<run>`",
            "",
            "Live run board (while a run is active): `make bench001-watch` → `artifacts/LIVE.md`",
            "",
        ]
    )
    path = progress_md_path()
    # Preserve manually curated Acc-lift / chunk ablation notes below the ladder.
    preserved = ""
    if path.exists():
        prev = path.read_text(encoding="utf-8")
        for marker in (
            "## LR Acc lift plan",
            "## Chunk Acc lift plan",
            "## Acc ablation notes",
        ):
            idx = prev.find(marker)
            if idx >= 0:
                preserved = "\n" + prev[idx:].rstrip() + "\n"
                break
    path.write_text("\n".join(lines) + preserved, encoding="utf-8")
    return path


def print_live(stage: str | None = None) -> int:
    """Print current LIVE.md (CLI ``live`` / Makefile watch)."""
    path = live_path(stage) if stage else live_root_path()
    if not path.exists() and stage is None:
        # Fall back to newest stage LIVE.md
        candidates = sorted(
            ARTIFACTS_DIR.glob("*/LIVE.md"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        if candidates:
            path = candidates[0]
    if not path.exists():
        print(f"No LIVE.md yet at {path}", flush=True)
        return 1
    text = path.read_text(encoding="utf-8")
    print(text, end="" if text.endswith("\n") else "\n")
    return 0


def _fmt(v: Any, *, signed: bool = False) -> str:
    if v is None:
        return "—"
    try:
        f = float(v)
    except (TypeError, ValueError):
        return str(v)
    return f"{f:+.4f}" if signed else f"{f:.4f}"
