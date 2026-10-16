# EdgeQuake vs LightRAG — Business Performance Report

**Generated:** 2026-10-16T00:26:47Z  
**Task:** GraphRAG-Bench Acc dual-SUT (July 2026 fair pins)  
**Profile:** `ACC_E2OCC_086_v1_lrlike_arms_v2`  
**Fixture:** `medical_publish_question_ids_v1` (n=200)  
**Valid run:** `True`

## One-screen first principles

```text
  Task     GraphRAG-Bench/EQ-vs-LR  (same corpus · questions · judge · Mix↔Mix)
  Sample   n=200 medical-mid (50/type) — bootstrap Acc CI is underpowered at smoke n=40; this is the defendable publish ladder before full core
  Acc      EQ 0.791 · LR 0.787 · Δ 0.004
  Δ Acc CI [-0.024, +0.031] (n=200) — includes 0 ⇒ tie
  L2       evidence recall EQ 0.893 / LR 0.947 · ctx_rel EQ 0.391 / LR 0.490
  Latency  query p50 EQ 5945 ms / LR 1315 ms
  Verdict  STATISTICAL TIE on answer quality
```

## Verdict

```text
  STATISTICAL TIE on answer quality
  Acc   EdgeQuake 0.791  ·  LightRAG 0.787  ·  Δ 0.004
  Δ Acc 95% CI: [-0.024, +0.031] (n=200) — includes 0 ⇒ tie
```

## What we tested

- Same GraphRAG-Bench medical questions (n=200) for both systems
- Same generator/judge stack: `mistral/mistral-small-latest` · embeddings `mistral/mistral-embed`
- Same Mix mode, matched top-k / chunk size, official `generation_eval` Acc
- Fairness: Mix arms always on, RRF fusion, chunk 1200/100, related_chunk=5
- L2 required: official `retrieval_eval` (evidence recall + context relevancy)
- **Not** UltraDomain win-rates · **not** paper Table-2 (GPT-4o-mini + BGE)
- **Why this n:** n=200 medical-mid (50/type) — bootstrap Acc CI is underpowered at smoke n=40; this is the defendable publish ladder before full core

## Scorecard for decisions

| Layer | Plain meaning | EdgeQuake | LightRAG | Winner |
|-------|---------------|-----------|----------|--------|
| Answer quality (Acc) | Are answers roughly as good? | 0.791 | 0.787 | Tie (CI) |
| Evidence coverage | Did we find the right sources? | 0.893 | 0.947 | LightRAG |
| Context cleanliness | Is the prompt low-noise? | 0.391 | 0.490 | LightRAG |
| Speed (query p50) | Time to answer (ms) | 5945 | 1315 | LightRAG |

- **EQ/LR p50 ratio:** 4.521× (product SLO target ≤ 1.5×)

## By question type (Acc)

| User need | EdgeQuake | LightRAG | Who leads |
|-----------|-----------|----------|-----------|
| Fact lookup | 0.756 | 0.756 | EdgeQuake |
| Multi-hop reasoning | 0.791 | 0.777 | EdgeQuake |
| Summarization | 0.832 | 0.839 | LightRAG |
| Creative / open-ended | 0.785 | 0.776 | EdgeQuake |

## July 2026 landscape

On this fair Acc head-to-head, EdgeQuake is a **LightRAG-class GraphRAG peer** when Acc is statistically tied. In the GraphRAG-Bench literature (ICLR 2026), **HippoRAG2-class** systems define the aspirational **retrieval SOTA** (high evidence recall **and** high context relevancy with compact prompts). Absolute Acc numbers from the academic paper use different models and are **not** directly comparable to these Mistral Acc pins.

## Allowed / forbidden external claims

| Allowed | Forbidden |
|---------|-----------|
| “Peer / statistical tie with LightRAG on Acc under fair pins” (or “point estimate ahead” only if CI excludes 0) | “Beats LightRAG” / “wins Acc” / “#1 GraphRAG-Bench” / “SOTA RAG” without CI excluding 0 and L2 gates |
| “Peer GraphRAG with production stack (Postgres, API, PDF pipeline)” | “#1 on GraphRAG-Bench” without matching paper protocol |
| “Actively closing retrieval noise / multi-hop / latency gaps” | Silent Acc headline = CE+protect without promotion gate |
| “Publish Acc on medical-mid n=200 under fair pins” | Publishing smoke n=40 as the release score |

## How to reproduce

```bash
make bench                 # medical-mid Acc (n=200) + this publish pack
make bench-warm            # query-only (auto latest warm EQ workspace)
make bench001-smoke-acc    # daily smoke gate only (n=40; not release)
```

- Pins: Mix arms on · RRF · chunk 1200/100 · top-k 30 · `mistral/mistral-small-latest` + `mistral/mistral-embed`

## Pointers

- **This publish pack:** `specs/001-benchmark/e2e/artifacts/publish/latest/`
- **Archive:** `specs/001-benchmark/e2e/artifacts/history/medical-mid-20261016T002647Z`
- **Technical SUMMARY:** same folder / archive `SUMMARY.md`
- **Static business brief:** `specs/001-benchmark/019-business-eq-vs-lightrag-and-rag.md`
- **Acc honesty close:** `specs/001-benchmark/001-edgquake-improvements/018-e4-acc-tie-close.md`
