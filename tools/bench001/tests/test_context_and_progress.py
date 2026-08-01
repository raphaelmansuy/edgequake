"""Accuracy gates: context export + progression ledger."""

from __future__ import annotations

import json
from pathlib import Path

from bench001.client import EdgeQuakeClient
from bench001.lightrag_runner import _lr_answer_and_context
from bench001.progress import (
    archive_run,
    begin_run,
    empty_context_rate,
    eta_seconds,
    format_duration,
    live_path,
    live_root_path,
    mark_phase,
    print_live,
    write_progress_md,
)


def test_eq_extract_answer_uses_snippet():
    client = EdgeQuakeClient("http://example.invalid")
    answer, ctx = client.extract_answer(
        {
            "answer": "MGZL needs hematopathologist review.",
            "sources": [
                {
                    "id": "c1",
                    "source_type": "chunk",
                    "score": 0.9,
                    "snippet": "Diagnosis of MGZL requires expert review.",
                },
                {
                    "id": "c2",
                    "source_type": "chunk",
                    "score": 0.8,
                    "content": "fallback content field",
                },
            ],
        }
    )
    assert "hematopathologist" in answer
    assert "Diagnosis of MGZL" in ctx
    assert "fallback content field" in ctx


def test_eq_extract_answer_empty_sources():
    client = EdgeQuakeClient("http://example.invalid")
    answer, ctx = client.extract_answer({"answer": "x", "sources": []})
    assert answer == "x"
    assert ctx == ""


def test_lr_answer_and_context_from_aquery_llm():
    answer, ctx = _lr_answer_and_context(
        {
            "llm_response": {"content": "Mediastinal gray zone lymphoma (MGZL)."},
            "data": {
                "chunks": [
                    {"content": "MGZL is a subtype of gray zone lymphomas."},
                    {"text": "Expert hematopathologist review is required."},
                ]
            },
        }
    )
    assert "MGZL" in answer
    assert "subtype" in ctx
    assert "hematopathologist" in ctx


def test_empty_context_rate():
    preds = [
        {"context": ["ok"]},
        {"context": [""]},
        {"context": []},
        {"context": "  "},
    ]
    assert abs(empty_context_rate(preds) - 0.75) < 1e-9


def test_format_duration_and_eta():
    assert format_duration(0) == "0s"
    assert format_duration(65) == "1m05s"
    assert format_duration(3661) == "1h01m01s"
    assert format_duration(None) == "—"
    assert eta_seconds(0, 10, 5) is None
    eta = eta_seconds(2, 10, 20.0)
    assert eta is not None and abs(eta - 80.0) < 1e-6
    assert eta_seconds(10, 10, 5.0) == 0.0


def test_progress_archive_and_ledger(tmp_path, monkeypatch):
    art = tmp_path / "artifacts"

    def _stage_dir(stage: str) -> Path:
        d = art / stage
        d.mkdir(parents=True, exist_ok=True)
        (d / "logs").mkdir(exist_ok=True)
        return d

    monkeypatch.setattr("bench001.progress.ARTIFACTS_DIR", art)
    monkeypatch.setattr("bench001.progress.stage_artifact_dir", _stage_dir)
    stage = "smoke"
    stage_dir = _stage_dir(stage)
    scorecard = {
        "stage": stage,
        "valid": False,
        "invalid_reason": "dry_run",
        "created_at_utc": "2026-07-19T00:00:00Z",
        "metrics": {
            "eq": {"overall_acc": 0.1},
            "lr": {"overall_acc": 0.2},
            "delta_eq_minus_lr": {"overall_acc": -0.1},
        },
        "ops": {
            "n_questions": 40,
            "eq_empty_context_rate": 0.0,
            "lr_empty_context_rate": 0.0,
        },
        "pins": {"judge": "rouge_proxy", "query_concurrency": 8, "eval_concurrency": 8},
    }
    (stage_dir / "scorecard.json").write_text(json.dumps(scorecard), encoding="utf-8")
    (stage_dir / "SUMMARY.md").write_text("# summary\n", encoding="utf-8")
    begin_run(stage, detail="unit-test")
    mark_phase(
        stage,
        "query_parallel",
        status="running",
        detail="EQ query 2/8",
        done=2,
        total=8,
        phase_elapsed_s=20.0,
    )
    live = live_path(stage).read_text(encoding="utf-8")
    assert "bench001 LIVE" in live
    assert "query_parallel" in live
    assert "ETA" in live
    assert live_root_path().exists()
    assert print_live(stage) == 0
    jsonl = stage_dir / "logs" / "progress.jsonl"
    assert jsonl.exists()
    assert "begin_run" in jsonl.read_text(encoding="utf-8")
    hist = archive_run(stage, scorecard)
    assert hist.exists()
    assert (hist / "scorecard.json").exists()
    assert (hist / "LIVE.md").exists()
    local_only = hist / "LOCAL_ONLY.md"
    assert local_only.exists()
    assert "SPEC-097" in local_only.read_text(encoding="utf-8")
    prog = write_progress_md()
    text = prog.read_text(encoding="utf-8")
    assert "SPEC-001 progression ladder" in text
    assert "0.1000" in text
    assert "bench001-watch" in text
