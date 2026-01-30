# OODA Iteration 03 - Decide

## Mission Re-Read ✅

**Mission**: Write 20+ promotional articles for EdgeQuake (including HN + Reddit)
**Spec File**: `./specs/006-write-articles.md`

---

## 🎯 Prioritized Actions

### Decision 1: Create Article 003 with All Platform Formats

**Deliverables**:

```
articles/003_entity_extraction_deep_dive/
├── medium.md      # 2000-2500 words, technical depth
├── linkedin.md    # <3000 characters
├── xcom.md        # 15 tweets
├── hackernews.md  # HN-style post
└── reddit.md      # Reddit post (r/MachineLearning, r/LocalLLaMA)
```

### Decision 2: Content Structure

**Title**: "How EdgeQuake Extracts Knowledge from Documents"
**Subtitle**: "LLMs as librarians: The entity extraction deep-dive"

**Core Sections**:

1. The problem: Why NER isn't enough
2. The solution: LLM-based extraction
3. The format: Why tuples beat JSON
4. The enhancement: Gleaning for completeness
5. The cleanup: Normalization for deduplication
6. Results: Before/after examples

### Decision 3: Key Visuals

1. **Extraction flow diagram** (text → LLM → tuples → graph)
2. **JSON vs Tuple failure** comparison
3. **Gleaning loop** visualization
4. **Normalization before/after**

---

## 📋 Action Checklist

- [ ] Create `articles/003_entity_extraction_deep_dive/`
- [ ] Write `medium.md` (2000-2500 words)
- [ ] Write `linkedin.md` (<3000 characters)
- [ ] Write `xcom.md` (15 tweets)
- [ ] Write `hackernews.md` (HN format)
- [ ] Write `reddit.md` (Reddit format)
- [ ] Verify technical accuracy against codebase
- [ ] Update act.md
