# OODA-14: Observe - RAG Evaluation with OpenAI

## Mission Re-Read Summary

**Mission file**: `specs/001-reliable-ingestion-mission.md`
**Iteration**: 14 of 50
**Focus**: RAG evaluation using OpenAI with EMILE_FREY documents

## Current System State

### Backend Configuration

```json
{
  "status": "healthy",
  "storage_mode": "postgresql",
  "providers": {
    "llm": {
      "name": "openai",
      "model": "gpt-5-nano"
    },
    "embedding": {
      "name": "openai",
      "model": "text-embedding-3-small",
      "dimension": 1536
    }
  },
  "pdf_storage_enabled": true
}
```

### OpenAI Quota Status

- **LLM API** (`/v1/responses`): ✅ WORKING
- **Embedding API** (`/v1/embeddings`): ✅ WORKING (user paid for quota)

### Test Documents Available (EMILE_FREY)

| Document                                 | Size    | Type      |
| ---------------------------------------- | ------- | --------- |
| C1 - Introduction IFRS 16.pdf            | 3.9 MB  | Training  |
| C2 - Formation Viareport.pdf             | 2.2 MB  | Training  |
| Module 1_Formation_CONSOHFM_v2025.pdf    | 1.2 MB  | Training  |
| Module 2_Formation_CONSOHFM_v2025.pdf    | 2.9 MB  | Training  |
| Note de fiscalité automobile.pdf         | 11.9 MB | Reference |
| Fiche pratique - Garantie conformité.pdf | 53 KB   | Reference |
| Fiche pratique - Vices cachés.pdf        | 75 KB   | Reference |
| Liste des DO & DONT.pdf                  | 157 KB  | Checklist |
| Conso - Echéanciers clients 2025.pdf     | 358 KB  | Financial |
| Fiscalité - Synthèse formalités.pdf      | 300 KB  | Tax       |

**Total**: 20+ documents covering:

- IFRS 16 accounting standards
- Viareport formations
- Automotive taxation
- Legal obligations (warranties, conformity)
- Financial schedules

## RAG Evaluation Objectives

### Phase 1: Document Ingestion

1. Upload 5 representative PDFs from EMILE_FREY
2. Verify entity extraction with gpt-5-nano
3. Measure ingestion time and token usage

### Phase 2: Query Quality Assessment

1. Ask domain-specific questions in French
2. Evaluate answer relevance and accuracy
3. Check source attribution

### Phase 3: Performance Metrics

1. Query response time
2. Entity recall
3. Answer completeness

## Initial Observations

### Current Document Count

```bash
curl -s http://localhost:8080/api/v1/documents | jq '.total'
```

### Documents to Upload for RAG Eval

1. `Fiche pratique - La garantie légale de conformité.pdf` (53 KB) - Simple legal doc
2. `Fiche pratique - L_action en garantie des vices cachés.pdf` (75 KB) - Legal reference
3. `C1 - Introduction IFRS 16.pdf` (3.9 MB) - Complex financial training
4. `Liste des DO & DONT.pdf` (157 KB) - Checklist format
5. `Note de fiscalité automobile - MAJ 16 avril 2025.pdf` (11.9 MB) - Comprehensive reference

## Next Steps

1. Upload the 5 selected documents
2. Wait for processing to complete
3. Execute RAG queries for evaluation
4. Document metrics and findings
