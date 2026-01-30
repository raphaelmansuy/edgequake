# Graph-RAG in the Real World: How Legal, Healthcare, and Finance Teams Extract Intelligence from Documents

_Vector search finds words. Graph-RAG finds relationships. Here's why that matters for regulated industries._

---

## The Query That Broke Vector Search

A partner at a top-tier law firm called me last year with a frustrating problem.

His team was doing M&A due diligence on an acquisition target. They had 50,000 contracts to review. Their state-of-the-art RAG system—built on vector embeddings—was returning thousands of results for every query.

"I searched for 'indemnification clauses' and got 2,000 hits," he said. "We need to review contracts where Party A has unlimited liability AND the termination clause allows exit without cause. How do I even ask that?"

He couldn't. Not with vector search.

**This is the fundamental limitation of baseline RAG: it finds similar text, not related concepts.**

When your query requires understanding relationships—who is liable to whom, under what conditions, with what exceptions—vector similarity fails.

Microsoft Research put it succinctly in their GraphRAG paper:

> "Baseline RAG struggles to connect the dots when answering a question requires traversing disparate pieces of information through their shared attributes."

This is exactly the problem Graph-RAG solves. And it's why regulated industries—legal, healthcare, and finance—are adopting it at scale.

---

## Legal: Contract Intelligence at Scale

### The Scenario

A private equity firm is acquiring a software company. The deal team needs to review 10,000 contracts inherited with the acquisition. They're looking for:

- Unlimited liability clauses (deal breakers above $50M)
- Termination provisions allowing exit without cause
- Change of control provisions that could trigger renegotiation

With vector search, each of these is a separate query returning hundreds of results. Cross-referencing them manually takes weeks.

### The Graph-RAG Approach

EdgeQuake builds a knowledge graph as it processes each contract:

```
┌─────────────────────────────────────────────────────────┐
│              CONTRACT KNOWLEDGE GRAPH                    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│   ┌───────────────────┐                                 │
│   │ CONTRACT_2024_001 │                                 │
│   │ "Software License │                                 │
│   │  Agreement"       │                                 │
│   └─────────┬─────────┘                                 │
│             │                                            │
│     ┌───────┼───────┬───────────────┐                   │
│     │       │       │               │                   │
│     ▼       ▼       ▼               ▼                   │
│ ┌───────┐ ┌───────┐ ┌──────────┐ ┌──────────────┐      │
│ │PARTY_A│ │PARTY_B│ │INDEMNITY │ │ TERMINATION  │      │
│ │"ACME" │ │"BETA" │ │UNLIMITED │ │ 30_DAYS_COC  │      │
│ └───┬───┘ └───┬───┘ └────┬─────┘ └──────┬───────┘      │
│     │         │          │              │               │
│     │         │          ▼              ▼               │
│     │         │    ┌──────────┐  ┌──────────────┐      │
│     │         │    │RISK_LEVEL│  │CHANGE_CONTROL│      │
│     │         │    │  "HIGH"  │  │  "TRIGGER"   │      │
│     └─────────┴────┴──────────┴──┴──────────────┘      │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

Now the partner can ask:

> "Find contracts where PARTY_A has INDEMNITY_UNLIMITED AND TERMINATION_COC AND RISK_LEVEL='HIGH'"

Result: 47 contracts. Reviewed in one afternoon instead of three weeks.

### Business Impact

| Metric               | Before GraphRAG      | After GraphRAG |
| -------------------- | -------------------- | -------------- |
| Precedent research   | 4 hours/clause       | 15 minutes     |
| Due diligence review | 3 weeks              | 3 days         |
| Missed risk signals  | Unknown              | 0 (validated)  |
| Cost per document    | $2+ (paralegal time) | $0.0014        |

---

## Healthcare: Clinical Knowledge Extraction

### The Scenario

A hospital system with 10 years of electronic health records wants to identify patients at risk for adverse drug events. Specifically:

- Patients with Type 2 diabetes
- Currently prescribed metformin
- Showing signs of declining kidney function (eGFR < 60)

Metformin is contraindicated in advanced kidney disease, but this combination often goes undetected in large patient populations.

### The Challenge

Clinical notes are unstructured. A typical note might read:

> "Patient presents with fatigue. Labs show eGFR 52, down from 68 six months ago. Currently on metformin 1000mg BID for T2DM. Consider nephrology consult."

Vector search on "metformin kidney" returns thousands of notes. What we need is _relationship-based_ filtering.

### The Graph-RAG Approach

EdgeQuake extracts entities and relationships from each clinical note:

```
CLINICAL_NOTE_2024_0157
    │
    ├──mentions──▶ PATIENT_ID_78452
    │
    ├──mentions──▶ DIABETES_TYPE_2
    │
    ├──mentions──▶ METFORMIN (1000mg BID)
    │
    ├──mentions──▶ EGFR_52 (declining)
    │
    └──mentions──▶ NEPHROLOGY_CONSULT (pending)

Relationships:
    METFORMIN ──contraindicated_with──▶ EGFR_BELOW_45
    EGFR_52 ──trending_toward──▶ EGFR_BELOW_45
```

Now the query becomes:

> "Find patients where MEDICATION=METFORMIN AND EGFR<60 AND EGFR_TREND=DECLINING"

Result: 234 patients flagged for pharmacist review.

### Compliance: On-Premise Processing

For healthcare, data sovereignty is non-negotiable. Protected Health Information (PHI) cannot leave the hospital network.

EdgeQuake integrates with Ollama for on-premise processing:

```rust
// Data never leaves your network
let provider = OllamaProvider::new("http://localhost:11434")
    .with_model("llama3:8b");

// Same pipeline, zero external API calls
edgequake.insert(clinical_note, Some(&note_id)).await?;
```

**After the initial hardware investment, every additional document costs $0.**

---

## Finance: Due Diligence Intelligence

### The Scenario

A PE fund is evaluating an acquisition. The target company has:

- 5 years of SEC filings (10-K, 10-Q, 8-K)
- 200 material contracts
- 50 board meeting minutes
- 100 investor presentations

The due diligence team needs to identify risk signals that predict trouble:

- Revenue recognition policy changes
- Executive departures (especially CFO)
- Auditor emphasis paragraphs
- Related party transactions

### The Challenge

These signals are scattered across different document types. A revenue recognition change appears in a 10-K footnote. The CFO departure is in an 8-K. The auditor concern is buried in the audit opinion. Vector search treats them as unrelated.

### The Graph-RAG Approach

EdgeQuake connects signals across documents:

```
┌─────────────────────────────────────────────────────────┐
│              FINANCIAL RISK INTELLIGENCE                 │
├─────────────────────────────────────────────────────────┤
│                                                          │
│   SEC_10K_FY2024 ───mentions───▶ REVENUE_RECOG_CHANGE   │
│        │                              │                  │
│        │                              ▼                  │
│        │                    ┌──────────────────┐        │
│        │                    │ Disclosed: Q3    │        │
│        │                    │ Impact: +$12M    │        │
│        │                    │ Reason: ASC 606  │        │
│        │                    └──────────────────┘        │
│        │                              │                  │
│   SEC_8K_20240215 ───mentions───────▶│                  │
│        │                              │                  │
│        ▼                              ▼                  │
│   ┌──────────────┐           ┌──────────────────┐       │
│   │CFO_DEPARTURE │◀─precedes─│AUDIT_EMPHASIS    │       │
│   │ 14 days prior│           │"Going concern"   │       │
│   └──────────────┘           └──────────────────┘       │
│        │                              │                  │
│        └──────────────┬───────────────┘                 │
│                       ▼                                  │
│              ┌─────────────────┐                        │
│              │  RISK_PATTERN   │                        │
│              │  Confidence: 94%│                        │
│              │  Similar to: 7  │                        │
│              │  past failures  │                        │
│              └─────────────────┘                        │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

The query:

> "Find companies where REVENUE_RECOGNITION_CHANGE within 90 days of CFO_DEPARTURE AND AUDIT_EMPHASIS='GOING_CONCERN'"

Result: 3 companies flagged. All three had material issues within 18 months.

### Business Impact

| Metric                | Manual Review      | EdgeQuake            |
| --------------------- | ------------------ | -------------------- |
| Time to insight       | 4 weeks            | 8 hours              |
| Documents reviewed    | 500 (sampled)      | 10,000 (all)         |
| Risk signals detected | 60% (estimated)    | 100%                 |
| Cost                  | $150K (consultant) | $140 (API + compute) |

---

## The Technical Pattern

Across legal, healthcare, and finance, the architecture is the same:

```
┌─────────────────────────────────────────────────────────┐
│              CROSS-INDUSTRY ARCHITECTURE                 │
├─────────────────────────────────────────────────────────┤
│                                                          │
│   Documents                                              │
│   ┌─────┐ ┌─────┐ ┌─────┐                               │
│   │ PDF │ │DOCX │ │ TXT │                               │
│   └──┬──┘ └──┬──┘ └──┬──┘                               │
│      │       │       │                                   │
│      └───────┴───────┘                                   │
│              │                                           │
│              ▼                                           │
│   ┌─────────────────────┐                               │
│   │ DOCUMENT INGESTION  │                               │
│   │ • Chunking          │                               │
│   │ • Text extraction   │                               │
│   └──────────┬──────────┘                               │
│              │                                           │
│              ▼                                           │
│   ┌─────────────────────┐                               │
│   │ ENTITY EXTRACTION   │ ◀─── LLM (OpenAI/Ollama)     │
│   │ • Named entities    │                               │
│   │ • Relationships     │                               │
│   │ • Properties        │                               │
│   └──────────┬──────────┘                               │
│              │                                           │
│              ▼                                           │
│   ┌─────────────────────┐                               │
│   │ KNOWLEDGE GRAPH     │                               │
│   │ PostgreSQL + AGE    │                               │
│   │ • Nodes             │                               │
│   │ • Edges             │                               │
│   │ • Embeddings        │                               │
│   └──────────┬──────────┘                               │
│              │                                           │
│              ▼                                           │
│   ┌─────────────────────┐                               │
│   │ QUERY ENGINE        │                               │
│   │ • Keyword extraction│                               │
│   │ • Graph traversal   │                               │
│   │ • Vector similarity │                               │
│   └──────────┬──────────┘                               │
│              │                                           │
│              ▼                                           │
│   ┌─────────────────────┐                               │
│   │ RESPONSE SYNTHESIS  │ ◀─── LLM                     │
│   │ • Context assembly  │                               │
│   │ • Answer generation │                               │
│   └─────────────────────┘                               │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## Why EdgeQuake for Regulated Industries

### 1. Data Sovereignty

Healthcare, legal, and finance all have strict data handling requirements. EdgeQuake supports on-premise deployment with Ollama:

```bash
# Start Ollama locally
ollama pull llama3:8b
ollama serve

# Point EdgeQuake to local model
export LLM_PROVIDER=ollama
export OLLAMA_BASE_URL=http://localhost:11434
make dev
```

**PHI, privileged communications, and material non-public information never leave your network.**

### 2. Audit Logging

Every query is logged for compliance:

```rust
// Built-in audit trail
audit_log.record(AuditEvent {
    user_id: "analyst_007",
    query: "Find contracts with unlimited liability",
    timestamp: Utc::now(),
    results_count: 47,
    document_ids: vec!["CONTRACT_001", "CONTRACT_023", ...],
});
```

### 3. Multi-Tenancy with Row-Level Security

Client data is isolated at the database level:

```sql
-- Each workspace completely isolated
ALTER TABLE entities ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_isolation ON entities
    USING (workspace_id = current_setting('app.workspace_id'));
```

---

## Getting Started

```bash
# Clone the repository
git clone https://github.com/raphaelmansuy/edgequake

# Start the full stack
make dev

# Process your first document
curl -X POST http://localhost:8080/api/v1/documents \
  -F "file=@contract.pdf"

# Query the knowledge graph
curl http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "Find contracts with unlimited liability"}'
```

---

## Conclusion

Vector search finds words. Graph-RAG finds relationships.

For regulated industries where understanding context matters—who is liable to whom, which treatments interact, what signals predict risk—knowledge graphs are not optional.

EdgeQuake brings Graph-RAG to these industries with:

- **Compliance-first architecture** (on-premise, audit logging, RLS)
- **Cost transparency** ($0.0014/document with gpt-4o-mini, $0 with Ollama)
- **Open source freedom** (Apache 2.0, no vendor lock-in)

The law firm partner? He's now doing due diligence in days instead of weeks. And he never gets 2,000 unrelated results.

---

## Acknowledgments

EdgeQuake implements the LightRAG algorithm from ["LightRAG: Simple and Fast Retrieval-Augmented Generation"](https://arxiv.org/abs/2410.05779). The Microsoft GraphRAG research paper ["From Local to Global: A Graph RAG Approach to Query-Focused Summarization"](https://arxiv.org/abs/2404.16130) provides foundational insights on graph-based retrieval.

---

_Which industry should we dive into deeper? Reply in the comments or reach out on GitHub._

---

**Tags**: #GraphRAG #Legal #Healthcare #Finance #RAG #KnowledgeGraph #EdgeQuake #Compliance #HIPAA #SOX
