---
title: 'Entity Extraction'
---

# Entity Extraction

> **Entity extraction uses LLMs to identify people, organizations, concepts, and
> their relationships from unstructured text.**

---

## What is Entity Extraction?

Entity extraction is the process of:

1. **Identifying** meaningful entities in text (people, places, concepts)
2. **Classifying** them by type (PERSON, ORGANIZATION, CONCEPT)
3. **Describing** their attributes based on context
4. **Connecting** them through explicit relationships

EdgeQuake uses LLMs as "knowledge engineers" — transforming unstructured text into structured knowledge.

---

## The Role of LLMs

Traditional NLP used rule-based extractors or trained models. EdgeQuake uses LLMs because:

| Approach        | Pros                               | Cons                     |
| --------------- | ---------------------------------- | ------------------------ |
| **Rules**       | Fast, predictable                  | Brittle, domain-specific |
| **Trained NER** | Accurate for known types           | Requires training data   |
| **LLM-based**   | Domain-agnostic, rich descriptions | Slower, requires API     |

LLM extraction provides:

- **Zero-shot extraction**: Works on any domain without training
- **Rich descriptions**: Not just labels, but context
- **Relationship inference**: Understands connections, not just entities

---

## Entity Types

EdgeQuake's default entity types:

```
┌─────────────────────────────────────────────────────────────────┐
│                    ENTITY TYPES                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  PERSON        │ People, characters, individuals                 │
│  ORGANIZATION  │ Companies, institutions, teams                  │
│  LOCATION      │ Places, regions, coordinates                    │
│  EVENT         │ Occurrences, meetings, milestones               │
│  CONCEPT       │ Ideas, theories, methods                        │
│  TECHNOLOGY    │ Tools, systems, platforms                       │
│  PRODUCT       │ Items, services, offerings                      │
│  OTHER         │ Fallback for uncategorized                      │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

**Custom types**: You can configure domain-specific types like `PROTEIN`, `DISEASE`, or `LEGAL_TERM`.

---

## The Extraction Process

```
┌─────────────────────────────────────────────────────────────────┐
│                EXTRACTION PIPELINE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────┐                                                   │
│  │   TEXT   │  "Dr. Sarah Chen leads the AI team at Quantum     │
│  │  CHUNK   │   Dynamics Lab. Her research on neural networks   │
│  │          │   has been cited 500 times."                      │
│  └────┬─────┘                                                   │
│       │                                                         │
│       v                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    LLM EXTRACTION                        │   │
│  │  System: "You are a Knowledge Graph Specialist..."       │   │
│  │  User: "Extract entities and relationships from..."      │   │
│  └──────────────────────────────────────────────────────────┘   │
│       │                                                         │
│       v                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    RAW OUTPUT (Tuples)                   │   │
│  │  entity<|#|>SARAH_CHEN<|#|>PERSON<|#|>Lead researcher... │   │
│  │  entity<|#|>QUANTUM_LAB<|#|>ORG<|#|>Research institution │   │
│  │  entity<|#|>NEURAL_NETWORKS<|#|>CONCEPT<|#|>ML approach  │   │
│  │  relation<|#|>SARAH_CHEN<|#|>QUANTUM_LAB<|#|>works_at... │   │
│  │  <|COMPLETE|>                                            │   │
│  └──────────────────────────────────────────────────────────┘   │
│       │                                                         │
│       v                                                         │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    PARSED RESULT                         │   │
│  │  Entities: 3                                             │   │
│  │  Relationships: 3                                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Relationship Extraction

Relationships connect entities with typed edges:

```
┌───────────────┐        WORKS_AT         ┌───────────────┐
│  SARAH_CHEN   │────────────────────────▶│  QUANTUM_LAB  │
│   (PERSON)    │                         │ (ORGANIZATION)│
└───────────────┘                         └───────────────┘
        │
        │ RESEARCHES
        │
        v
┌───────────────┐
│NEURAL_NETWORKS│
│   (CONCEPT)   │
└───────────────┘
```

Each relationship includes:

- **Source entity**: Starting node
- **Target entity**: Ending node
- **Type/Keywords**: Relationship category
- **Description**: Context from the text

---

## Entity Normalization

Before storing, entity names are normalized to prevent duplicates:

| Raw Input       | Normalized Output |
| --------------- | ----------------- |
| `"John Doe"`    | `JOHN_DOE`        |
| `"john doe"`    | `JOHN_DOE`        |
| `"the Company"` | `COMPANY`         |
| `"John's team"` | `JOHN_TEAM`       |

**Why normalize?**

- Prevents "John Doe" and "john doe" becoming separate nodes
- Enables entity merging across documents
- Improves query accuracy

See [normalizer.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-pipeline/src/prompts/normalizer.rs) for implementation.

---

## Tuple vs JSON Format

EdgeQuake uses tuple-delimited format by default:

```
entity<|#|>NAME<|#|>TYPE<|#|>DESCRIPTION
```

**Why not JSON?**

| Aspect           | Tuple Format         | JSON Format                |
| ---------------- | -------------------- | -------------------------- |
| Streaming        | ✅ Line-by-line      | ❌ Need complete structure |
| Partial recovery | ✅ Parse valid lines | ❌ All or nothing          |
| LLM reliability  | ✅ Fewer errors      | ❌ Escaping issues         |

---

## Gleaning: Multi-Pass Extraction

LLMs sometimes miss entities. Gleaning performs a second pass:

```
Pass 1: "Extract entities from this text..."
Result: SARAH_CHEN, QUANTUM_LAB

Pass 2: "What entities did you miss? Already found: SARAH_CHEN, QUANTUM_LAB"
Result: NEURAL_NETWORKS, AI_RESEARCH (missed in first pass)
```

Research shows 1-2 gleaning iterations improve recall by 15-25%.

---

## Learn More

- **Where entities are stored**: [Knowledge Graph](/docs/concepts/knowledge-graph/)
- **Algorithm details**: [LightRAG Algorithm](/docs/deep-dives/lightrag-algorithm/)
- **How queries use entities**: [Hybrid Retrieval](/docs/concepts/hybrid-retrieval/)

---

## Source Code

- **Extraction logic**: [extractor.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-pipeline/src/extractor.rs)
- **Prompts**: [entity_extraction.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-pipeline/src/prompts/entity_extraction.rs)
- **Normalization**: [normalizer.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-pipeline/src/prompts/normalizer.rs)
- **Parsing**: [parser.rs](https://github.com/raphaelmansuy/edgequake/blob/edgequake-main/edgequake/crates/edgequake-pipeline/src/prompts/parser.rs)
