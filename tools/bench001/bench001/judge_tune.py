"""Judge / Acc tuning helpers for SPEC-001.

Acc (GraphRAG-Bench) = factuality_weight * F1 + (1-w) * embed_cosine(answer, gold).

First principles (Acc lift while keeping a Mistral judge — Jul 2026):
  1. Acc is F1-heavy → state every context fact that answers the question. A one-sentence cap drops a later clause that was already retrieved.
  2. Do not refuse when evidence is present (over-refusal zeros Acc).
  3. Keep SUT embed / corpus / top-k fixed when changing style (one confound).
  4. Prefer a stronger *same-family* judge for statement decomposition
     (e.g. mistral-medium-latest) — Mistral Studio guidance for nuanced judges.
  5. Never raise Acc by dropping L2 retrieval or hiding empty context.
  6. Report Acc under a named answer_style pin; do not silently change prompts.

Paper parity Acc still needs GPT-4o-mini + BGE (labeled P0_paper) — orthogonal.
"""

from __future__ import annotations

import os
from typing import Any

# Upstream GraphRAG-Bench Acc weights (answer_accuracy.py).
DEFAULT_ACC_FACTUALITY_WEIGHT = 0.75
DEFAULT_JUDGE_TEMPERATURE = 0.0

# Paper-parity metric embed (Table 2 / eval README).
PAPER_JUDGE_EMBEDDING_MODEL = "BAAI/bge-large-en-v1.5"

# Same-family stronger judge for statement F1 (nuanced eval); SUT may stay Small.
RECOMMENDED_MISTRAL_JUDGE_MODEL = "mistral-medium-latest"

CONCISE_SYSTEM_PROMPT = (
    "Answer the question using only the retrieved context. "
    "Be concise: prefer 1–3 short sentences (or a short bullet list). "
    "Do not invent acronym expansions. Do not add background essays, "
    "hedging, or markdown headings unless essential. "
    "If the answer is a single fact, state that fact directly."
)

# Citation-free answer rules. Completeness matches LightRAG rag_response
# ("extract all pieces of information that are directly relevant"). A one-sentence
# cap is not part of that prompt and drops a second clause that is already in context.
GOLD_SYSTEM_PROMPT = (
    "You are answering a GraphRAG-Bench medical question for accuracy scoring.\n"
    "Rules (strict):\n"
    "1) Use ONLY the retrieved context. Prefer facts that appear in the context.\n"
    "2) State every context fact that answers the question. When the context gives "
    "alternatives, conditions, or a second clause, include them. Do not stop after "
    "the first supporting fact. No markdown headings and no preamble.\n"
    "3) Do NOT say 'Not answerable' / 'I cannot' if ANY context chunk is relevant "
    "to the question — answer with the facts that context supports.\n"
    "4) Do NOT expand acronyms unless the context expands them; keep entity names "
    "as written in the context.\n"
    "5) Do NOT add caveats, differential diagnoses, or 'consult a specialist' "
    "unless the question asks for them.\n"
    "6) When the question asks what applies, name each item the context states. "
    "Do not collapse a list into a single example.\n"
    "7) Do NOT append citation markers, chunk ids, or brackets like [1], [16], "
    "(source), or 'according to chunk' — plain answer text only (gold has none)."
)


def answer_style() -> str:
    raw = (os.environ.get("BENCH001_ANSWER_STYLE") or "gold").strip().lower()
    if raw in {"gold", "concise", "default", "verbose"}:
        return raw
    return "gold"


def concise_prompt_enabled() -> bool:
    """True when a style system/user prompt should be injected."""
    return answer_style() in {"gold", "concise"}


def system_prompt_for_style() -> str | None:
    style = answer_style()
    if style == "gold":
        return GOLD_SYSTEM_PROMPT
    if style == "concise":
        return CONCISE_SYSTEM_PROMPT
    return None


def judge_temperature(override: float | None = None) -> float:
    if override is not None:
        return max(0.0, min(2.0, float(override)))
    raw = os.environ.get("BENCH001_JUDGE_TEMPERATURE")
    if raw is None or not str(raw).strip():
        return DEFAULT_JUDGE_TEMPERATURE
    try:
        return max(0.0, min(2.0, float(raw)))
    except ValueError:
        return DEFAULT_JUDGE_TEMPERATURE


def acc_factuality_weight(override: float | None = None) -> float:
    if override is not None:
        return max(0.0, min(1.0, float(override)))
    raw = os.environ.get("BENCH001_ACC_FACTUALITY_WEIGHT")
    if raw is None or not str(raw).strip():
        return DEFAULT_ACC_FACTUALITY_WEIGHT
    try:
        return max(0.0, min(1.0, float(raw)))
    except ValueError:
        return DEFAULT_ACC_FACTUALITY_WEIGHT


def export_judge_env(
    *,
    temperature: float | None = None,
    factuality_weight: float | None = None,
    embed_backend: str | None = None,
    embed_base_url: str | None = None,
) -> dict[str, str]:
    """Env exported into the generation_eval subprocess."""
    t = judge_temperature(temperature)
    w = acc_factuality_weight(factuality_weight)
    backend = (embed_backend or os.environ.get("BENCH001_JUDGE_EMBED_BACKEND") or "auto").strip()
    out = {
        "BENCH001_JUDGE_TEMPERATURE": str(t),
        "BENCH001_ACC_FACTUALITY_WEIGHT": str(w),
        "BENCH001_JUDGE_EMBED_BACKEND": backend,
    }
    if embed_base_url:
        out["BENCH001_JUDGE_EMBED_BASE_URL"] = embed_base_url
    elif os.environ.get("BENCH001_JUDGE_EMBED_BASE_URL"):
        out["BENCH001_JUDGE_EMBED_BASE_URL"] = os.environ["BENCH001_JUDGE_EMBED_BASE_URL"]
    return out


def judge_tune_pin_fields() -> dict[str, Any]:
    return {
        "judge_temperature": judge_temperature(),
        "judge_acc_factuality_weight": acc_factuality_weight(),
        "judge_embed_backend": os.environ.get("BENCH001_JUDGE_EMBED_BACKEND", "auto"),
        "answer_style": answer_style(),
        "recommended_mistral_judge": RECOMMENDED_MISTRAL_JUDGE_MODEL,
        "acc_formula": "0.75*F1 + 0.25*embed_cosine (weights overridable)",
        "acc_lift_note": (
            "Closer Acc under Mistral judge: answer_style=gold (SUT) + "
            f"judge_model={RECOMMENDED_MISTRAL_JUDGE_MODEL}; keep L2 gates"
        ),
    }
