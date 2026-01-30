# Vector Search Fails at This. Graph-RAG Doesn't.

A law firm searched their 50,000 contracts for "indemnification clauses."

Result: 2,000 documents. Too many to review.

What they actually needed: "Contracts where Party A has unlimited liability AND termination allows exit without cause."

**Vector search can't do this. It finds similar text, not related concepts.**

---

**Three industries where Graph-RAG wins:**

**🏛️ Legal**

- Query: "Unlimited liability + termination without cause"
- Result: 47 contracts (from 50,000)
- Time saved: 3 weeks → 3 days

**🏥 Healthcare**

- Query: "Diabetes + metformin + declining kidney function"
- Result: 234 patients flagged for pharmacist review
- Compliance: All processing on-premise (Ollama)

**📊 Finance**

- Query: "Revenue recognition change + CFO departure + auditor emphasis"
- Result: 3 companies flagged, all had issues within 18 months
- Coverage: 10,000 documents reviewed vs 500 sampled

---

**Why Graph-RAG works:**

Vector search: "Find documents containing these words"
Graph-RAG: "Find entities with these relationships"

The difference is multi-hop reasoning:

```
Contract → has_clause → INDEMNIFICATION_UNLIMITED
Contract → has_clause → TERMINATION_30_DAYS
INDEMNIFICATION_UNLIMITED → risk_level → HIGH
```

---

**EdgeQuake brings this to regulated industries:**

✅ On-premise deployment (HIPAA, SOX, GDPR)
✅ Audit logging for compliance
✅ Row-level security for multi-tenancy
✅ $0.0014/document (or $0 with Ollama)

Open source, Apache 2.0: github.com/raphaelmansuy/edgequake

---

Which industry would you like us to explore deeper?

#GraphRAG #LegalTech #HealthIT #FinTech #RAG #AI
