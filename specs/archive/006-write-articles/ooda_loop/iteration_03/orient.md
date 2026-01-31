# OODA Iteration 03 - Orient

## Mission Re-Read ✅

**Mission**: Write 20+ promotional articles for EdgeQuake
**Spec File**: `./specs/006-write-articles.md`

---

## 🧭 Analysis for Article 003

### Article Positioning

**Narrative Arc**:

```
Article 001: "Classic RAG is broken"
     │
Article 002: "EdgeQuake fixes it with 3 stages"
     │
Article 003: "Here's how the extraction magic works" ← YOU ARE HERE
```

### Target Reader Profile

**Primary**: ML Engineers implementing Graph-RAG
**Secondary**: CTOs evaluating technical depth
**Tertiary**: Researchers comparing approaches

### Key Technical Insights to Highlight

1. **LLM as Extraction Engine**
   - No custom NER models needed
   - Domain-agnostic (works on any text)
   - Rich semantic descriptions

2. **Tuple Format Robustness**
   - JSON parsing fails 10-20% with LLMs
   - Tuple format: line-by-line, partial recovery
   - Production-proven from LightRAG

3. **Gleaning for Completeness**
   - Multi-pass extraction
   - 20-30% more entities discovered
   - Configurable depth

4. **Normalization for Deduplication**
   - "John Doe" → "JOHN_DOE"
   - 67% deduplication in practice
   - Clean, consistent graphs

### Content Strategy

**Feynman Technique**: Use the "librarian" analogy

- Reading documents = scanning text
- Highlighting entities = using a highlighter
- Drawing connections = sticky notes
- Organizing = normalization

### ASCII Diagrams Needed

1. **Extraction flow**: Text → LLM → Tuples → Graph
2. **JSON vs Tuple failure comparison**
3. **Gleaning loop visualization**
4. **Before/After normalization**

### Platform-Specific Considerations

| Platform   | Focus                          | Hook                              |
| ---------- | ------------------------------ | --------------------------------- |
| Medium     | Technical depth, code snippets | "How LLMs become librarians"      |
| LinkedIn   | Business value, ROI            | "20-30% more knowledge from docs" |
| X.com      | Visual, shareable              | "The JSON trap"                   |
| HackerNews | Technical novelty, skepticism  | "Why we ditched JSON parsing"     |
| Reddit     | Community value                | "Show HN"-style                   |

---

## Risks and Mitigations

| Risk                       | Mitigation                    |
| -------------------------- | ----------------------------- |
| Too implementation-focused | Balance with business value   |
| LLM cost concerns          | Address in cost article (011) |
| "Just use NER" objections  | Compare coverage/quality      |
