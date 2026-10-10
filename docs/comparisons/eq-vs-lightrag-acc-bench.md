---
title: 'EQ vs LightRAG — Acc Bench (SPEC-001)'
description: Measured GraphRAG-Bench Acc, fair cold latency, and labeled warm cache latency for EdgeQuake vs LightRAG.
---

# EdgeQuake vs LightRAG: Acc Bench (SPEC-001)

This page reports the measured head-to-head between EdgeQuake and LightRAG on answer accuracy and query latency. It is the single source of truth (SSOT) for these numbers, for anyone who needs to quote them. Do not change a measured value here without a new benchmark run.

**In one line:** on accuracy the two systems are a statistical tie. LightRAG retrieves slightly better evidence. Cold latency is about equal. With a warm cache, EdgeQuake answers faster.

Terms used below:

- **Acc** is answer accuracy as scored by the benchmark's official `generation_eval`.
- **p50** is the median time for a query.
- **CI** is a 95% confidence interval. If the CI for the difference includes 0, the result is a tie.
- **Cold** means no LLM cache. **Warm** means the LLM and embedding caches are on for both systems.

> Publish Acc SSOT: GraphRAG-Bench medical-mid, run with `make bench`. `publish/latest/` is the archive `medical-mid-20260815T110218Z` (2026-08-15). The medical-full peer (n=2062) ran on 2026-08-16. It is a labeled scale check, not the SSOT.

The test is fair and dual-system: same corpus, same questions, Mix mode, Mistral Small plus `mistral-embed`, and the official `generation_eval`. It is **not** the UltraDomain win-rate test, and it is **not** the paper's Table 2 (GPT-4o-mini plus BGE).

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart LR
    corpus["Same corpus and questions"] --> eq["EdgeQuake, Mix mode"]
    corpus --> lr["LightRAG, Mix mode"]
    eq --> eval["Official generation_eval"]
    lr --> eval
    eval --> acc["Acc and evidence scores"]
    eq --> lat["Latency timing"]
    lr --> lat
```

Read it left to right. Both systems get the same inputs and the same scorer, so differences come from the systems.

Protocol: [SPEC-001 index](../../specs/001-benchmark/000-index.md). Business brief: [019](../../specs/001-benchmark/019-business-eq-vs-lightrag-and-rag.md).

---

## One-screen verdict

| Layer             | EdgeQuake | LightRAG  | Claim                                                                 |
| ----------------- | --------- | --------- | --------------------------------------------------------------------- |
| **Acc** (n=200)   | **0.792** | **0.786** | Statistical tie (Δ CI includes 0); **not Acc Beat**                   |
| **Acc** (n=2062)  | **0.786** | **0.786** | Point tie on Acc-law full (peer `ACC_E2OCC_086_MEDICAL_FULL_v1`); **not Beat** |
| Evidence recall   | 0.932     | 0.949     | LightRAG                                                              |
| Context relevancy | 0.471     | 0.510     | LightRAG (`ctx_rel` still below the 0.50 promote gate)                      |
| Cold query p50    | 4447 ms   | 4359 ms   | **≈ tied (1.02×)** (peer `C1COLD_v1`, unchanged)                      |
| Warm cache p50    | **82 ms** | 993 ms    | **EQ 0.083×** — labeled peer `EQ_LLM_CACHE_WARM_v1` (LLM+embed cache) |

**Do not claim** “EdgeQuake beats LightRAG” on **Acc**, Acc mid Parity, or SOTA on GraphRAG-Bench. Acc Beat / Acc Equal mid remain **STOP** until [080 promote checklist](../../specs/001-benchmark/001-edgquake-improvements/080-phase-g-promote-checklist.md) is green (`ctx_rel ≥ 0.50`, Fact ER ≥ LR−0.03, medical-full). Acc ingest pin is **chunk 1200/100** (LR parity).

**Allowed warm claim:** under matched LLM+embed cache, warm Mix wall **EdgeQuake beats LightRAG** (82 vs 993 ms). Fair cold engine truth stays `C1COLD_v1`.

---

## Publish Acc (medical-mid n=200)

| Field | Value |
|-------|--------|
| Archive | [`medical-mid-20260815T110218Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20260815T110218Z/) |
| Publish pack | [`publish/latest/`](../../specs/001-benchmark/e2e/artifacts/publish/latest/) |
| Profile | `ACC_E2OCC_086_v1_lrlike_arms_v2` |
| Fixture | `medical_publish_question_ids_v1` (50 questions / type) |
| Δ Acc 95% CI | **[-0.022, +0.034]** (bootstrap; CI includes 0 ⇒ tie) |
| Valid | `true` |
| `can_claim_beats_lightrag` | `false` (Acc CI includes 0; L2 ctx / Fact ER promote gates unmet) |

### By question type (Acc)

| Type | EdgeQuake | LightRAG | Lead |
|------|-----------|----------|------|
| Fact Retrieval | 0.779 | 0.756 | EdgeQuake |
| Complex Reasoning | 0.797 | 0.778 | EdgeQuake |
| Contextual Summarize | 0.814 | 0.844 | LightRAG |
| Creative Generation | 0.776 | 0.766 | EdgeQuake |

### Ops (Acc pack)

- Empty-answer rates: EQ 0% · LR 0%
- Ingest wall: warm query-only (full-corpus workspace `23b09c73-…`)
- Acc pack latency is **not** the warm product claim — Acc pins `EDGEQUAKE_LLM_CACHE=0`; LR warm looks faster only with LLM cache ([063](../../specs/001-benchmark/001-edgquake-improvements/063-why-lightrag-faster-cache-fairness.md))

---

## Fair cold latency peer

| Field | Value |
|-------|--------|
| Peer | [`C1COLD_v1`](../../specs/001-benchmark/e2e/artifacts/publish/peers/C1COLD_v1/) |
| Archive | [`smoke-20260723T134452Z`](../../specs/001-benchmark/e2e/artifacts/history/smoke-20260723T134452Z/) |
| Pin | `BENCH001_LR_ENABLE_LLM_CACHE=0` · query-only · n=40 |
| EQ/LR p50 | 4447 / 4359 ms → **1.02×** (SLO ≤1.5× **PASS**) |

---

## Labeled warm latency peer (product cache)

| Field | Value |
|-------|--------|
| Peer | [`EQ_LLM_CACHE_WARM_v1`](../../specs/001-benchmark/e2e/artifacts/publish/peers/EQ_LLM_CACHE_WARM_v1/) |
| Archive | [`medical-mid-20260815T132034Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20260815T132034Z/) |
| Pin | EQ `EDGEQUAKE_LLM_CACHE=1` · LR `enable_llm_cache=True` · n=200 · `SKIP publish/latest` |
| EQ/LR p50 | **82 / 993 ms → 0.083×** |
| EQ stages p50 | keyword **0** · embed **0** · retrieve **56** · generate **0** |
| Acc on pack | EQ 0.792 / LR 0.773 — statistical **tie**; **not Acc Beat** |
| Law | Workspace Mix inject reuses engine embed LRU ([064](../../specs/001-benchmark/001-edgquake-improvements/064-product-ttft-cache-batch-embed.md)) |

Prior Acc packs: [`medical-mid-20260815T090820Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20260815T090820Z/) (EQ 0.805 / LR 0.798) · [`medical-mid-20260812T004216Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-mid-20260812T004216Z/) (EQ 0.783 / LR 0.774).

---

## Acc-law medical-full (n=2062, labeled scale, not Acc SSOT)

Best known Acc pack (E2-occ 086, chunk 1200/100, GWC off, pool=mix, query-only on `23b09c73-…`).

| Field | Value |
|-------|--------|
| Peer | [`ACC_E2OCC_086_MEDICAL_FULL_v1`](../../specs/001-benchmark/e2e/artifacts/publish/peers/ACC_E2OCC_086_MEDICAL_FULL_v1/) |
| Archive | [`medical-full-20260816T012004Z`](../../specs/001-benchmark/e2e/artifacts/history/medical-full-20260816T012004Z/) |
| Ingest pin | chunk **1200/100** (LR CHUNK_SIZE parity) |
| Acc | EQ **0.786** · LR **0.786** (point tie; bootstrap paired n=16 underpowered) |
| ctx_rel | EQ 0.427 · LR 0.485 — still **below 0.50** |
| overall ER / Fact ER | 0.927 / **0.914** vs LR 0.947 / 0.945 |
| vs P0 full (Jul 22) | Acc 0.724 → **0.786** (closed 6pp scale gap) |
| `can_claim_beats_lightrag` | `false` |

---

## Allowed / forbidden claims

| Allowed | Forbidden |
|---------|-----------|
| “Peer / statistical tie with LightRAG on Acc under fair Mistral pins (medical-mid n=200); L2 still trails LightRAG” | “Beats LightRAG” on **Acc** / Acc Beat / Acc Equal mid Parity |
| “Fair cold Mix latency ≈ LightRAG (~1.02×)” | Treating warm Acc mid (CACHE=0 vs LR cache-on) as engine win |
| “Warm Mix with LLM+embed cache: EQ 82 ms vs LR 993 ms (0.083×)” — peer `EQ_LLM_CACHE_WARM_v1` | Replacing Acc `publish/latest` with the warm peer |
| “Product query API Equal LightRAG” ([083](../../specs/001-benchmark/001-edgquake-improvements/083-lightrag-query-api-law.md)) | Merging Acc, L2, latency, and product peers into one unlabeled winner |
| “Publish Acc on medical-mid n=200; Acc-law full n=2062 is a **point tie** (labeled, chunk 1200/100)” | Publishing smoke n=40 as the release score |

Machine index: [`peers.json`](../../specs/001-benchmark/e2e/artifacts/peers.json).

---

## Reproduce

```bash
export MISTRAL_API_KEY=...
make bench001-doctor
BENCH001_ALLOW_PUBLISH_LATEST=1 make bench   # medical-mid n=200 + publish/latest

# Fair cold latency (does not overwrite Acc publish/latest):
export BENCH001_EQ_WORKSPACE_ID=<warm-full-corpus-ws>
export BENCH001_SKIP_PUBLISH_LATEST=1
export BENCH001_PUBLISH_PEER=C1COLD_v1
make bench001-c1cold

# Labeled warm product latency (does not overwrite Acc publish/latest):
make bench001-medical-mid-eq-llm-cache-warm

# Acc-law medical-full n=2062 (does not overwrite Acc publish/latest):
export BENCH001_EQ_WORKSPACE_ID=23b09c73-aa3f-4497-8e11-c448ffad8c53
export BENCH001_QUERY_ONLY=1 BENCH001_SKIP_PUBLISH_LATEST=1
export BENCH001_PUBLISH_PEER=ACC_E2OCC_086_MEDICAL_FULL_v1
python3 -m bench001.cli medical-full --api http://127.0.0.1:8090 --query-only \
  --profile-id ACC_E2OCC_086_v1 --query-concurrency 4 --eval-concurrency 24
```

Stakeholder pack after `make bench` with publish allow:

- [BUSINESS_REPORT.md](../../specs/001-benchmark/e2e/artifacts/publish/latest/BUSINESS_REPORT.md)
- [EXEC_SUMMARY.txt](../../specs/001-benchmark/e2e/artifacts/publish/latest/EXEC_SUMMARY.txt)
- [SUMMARY.md](../../specs/001-benchmark/e2e/artifacts/publish/latest/SUMMARY.md)

Warm latency peer pack:

- [EQ_LLM_CACHE_WARM_v1/BUSINESS_REPORT.md](../../specs/001-benchmark/e2e/artifacts/publish/peers/EQ_LLM_CACHE_WARM_v1/BUSINESS_REPORT.md)
- [EQ_LLM_CACHE_WARM_v1/EXEC_SUMMARY.txt](../../specs/001-benchmark/e2e/artifacts/publish/peers/EQ_LLM_CACHE_WARM_v1/EXEC_SUMMARY.txt)

Acc-law medical-full peer pack:

- [ACC_E2OCC_086_MEDICAL_FULL_v1/BUSINESS_REPORT.md](../../specs/001-benchmark/e2e/artifacts/publish/peers/ACC_E2OCC_086_MEDICAL_FULL_v1/BUSINESS_REPORT.md)
- [ACC_E2OCC_086_MEDICAL_FULL_v1/README.md](../../specs/001-benchmark/e2e/artifacts/publish/peers/ACC_E2OCC_086_MEDICAL_FULL_v1/README.md)