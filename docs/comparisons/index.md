---
title: Comparisons
description: Fair, sourced comparisons of EdgeQuake with LightRAG, Microsoft GraphRAG, and plain vector RAG, including the published benchmark.
---

# Comparisons

These pages compare EdgeQuake with other ways to build RAG. They are for people choosing a tool. Each page says what is measured, what is only a design difference, and what we have not verified.

The one measured result is against LightRAG: a statistical tie on accuracy. Other pages compare design and features only.

| Page | What it covers |
| ---- | -------------- |
| [Acc benchmark](./eq-vs-lightrag-acc-bench.md) | The measured accuracy and latency results against LightRAG. This is the source of truth for those numbers. |
| [vs LightRAG (Python)](./vs-lightrag-python.md) | EdgeQuake and the Python original: modes, storage, and operations |
| [vs GraphRAG](./vs-graphrag.md) | Microsoft GraphRAG: communities and reports versus relationship search |
| [vs traditional RAG](./vs-traditional-rag.md) | What a graph adds to vector-only search, and what it costs |

## Which page to read

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
    Q["What do you want to know?"] --> A{"Measured accuracy against LightRAG?"}
    A -->|yes| B["Acc benchmark"]
    A -->|no| C{"Compared with another RAG system?"}
    C -->|LightRAG Python| D["vs LightRAG Python"]
    C -->|Microsoft GraphRAG| E["vs GraphRAG"]
    C -->|plain vector RAG| F["vs traditional RAG"]
```

Start at the top of the chart and follow the first question that matches your need.

## Historical

- [Implementation differences, Feb 2026 snapshot](./edgequake-vs-lightrag-superiority-analysis.md): an old code audit, corrected and made neutral. Prefer the pages above.
