"""Judge / Acc tuning helpers."""

from bench001.judge_tune import (
    GOLD_SYSTEM_PROMPT,
    RECOMMENDED_MISTRAL_JUDGE_MODEL,
    acc_factuality_weight,
    answer_style,
    export_judge_env,
    judge_temperature,
    system_prompt_for_style,
)
from bench001.profiles import PAPER_JUDGE_EMBEDDING_MODEL


def test_defaults_gold_and_weights(monkeypatch):
    monkeypatch.delenv("BENCH001_ANSWER_STYLE", raising=False)
    monkeypatch.delenv("BENCH001_JUDGE_TEMPERATURE", raising=False)
    monkeypatch.delenv("BENCH001_ACC_FACTUALITY_WEIGHT", raising=False)
    assert answer_style() == "gold"
    assert system_prompt_for_style() == GOLD_SYSTEM_PROMPT
    assert judge_temperature() == 0.0
    assert abs(acc_factuality_weight() - 0.75) < 1e-9
    assert "medium" in RECOMMENDED_MISTRAL_JUDGE_MODEL


def test_tune_overrides(monkeypatch):
    monkeypatch.setenv("BENCH001_ANSWER_STYLE", "default")
    monkeypatch.setenv("BENCH001_JUDGE_TEMPERATURE", "0.2")
    monkeypatch.setenv("BENCH001_ACC_FACTUALITY_WEIGHT", "0.9")
    assert answer_style() == "default"
    assert system_prompt_for_style() is None
    assert abs(judge_temperature() - 0.2) < 1e-9
    assert abs(acc_factuality_weight() - 0.9) < 1e-9
    env = export_judge_env(temperature=0.0, factuality_weight=0.5, embed_backend="openai_compat")
    assert env["BENCH001_JUDGE_TEMPERATURE"] == "0.0"
    assert env["BENCH001_ACC_FACTUALITY_WEIGHT"] == "0.5"
    assert env["BENCH001_JUDGE_EMBED_BACKEND"] == "openai_compat"


def test_concise_still_available(monkeypatch):
    monkeypatch.setenv("BENCH001_ANSWER_STYLE", "concise")
    assert answer_style() == "concise"
    assert system_prompt_for_style() is not None
    assert "concise" in (system_prompt_for_style() or "").lower()


def test_paper_embed_constant():
    assert PAPER_JUDGE_EMBEDDING_MODEL == "BAAI/bge-large-en-v1.5"


def test_gold_prompt_keeps_every_supporting_context_fact():
    text = GOLD_SYSTEM_PROMPT
    assert "ONE short sentence" not in text
    assert "that fact only" not in text
    assert "every context fact that answers the question" in text
    assert "Do not stop after the first supporting fact" in text
    assert "plain answer text only" in text
