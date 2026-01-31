# Iteration 09: Orient - Entity Deduplication Analysis

## Target Audience Analysis

### Technical Leaders (CTO/VP Engineering)

**WHY they care**: Data quality directly impacts:

- Answer accuracy (duplicates = fragmented knowledge)
- Storage costs (N duplicates = N× storage)
- Query performance (larger graph = slower traversal)

**Message focus**: Data quality at scale, cost efficiency

### ML/AI Engineers

**WHY they care**:

- Graph fragmentation ruins relationship queries
- Debugging "why didn't it find X?" often leads to duplicates
- Entity resolution is a classic NLP challenge

**Message focus**: Normalization rules, merge strategies

### Data Engineers

**WHY they care**:

- Data cleaning is 80% of their job
- Entity resolution at scale is expensive
- Need deterministic, explainable rules

**Message focus**: Deterministic normalization, lineage tracking

## Key Pain Points Deduplication Solves

### Pain Point 1: "Same Entity, Multiple Nodes"

**Traditional approach**: Store whatever the LLM outputs
**EdgeQuake solution**: Normalize before storage

- "John Doe" → "JOHN_DOE"
- "john doe" → "JOHN_DOE"
- All merge to same node

### Pain Point 2: "Information Gets Lost"

**Traditional approach**: Replace old description with new
**EdgeQuake solution**: Merge descriptions

- Document 1: "Chen is an engineer"
- Document 2: "Chen leads the ML team"
- Merged: "Chen is an engineer and leads the ML team"

### Pain Point 3: "Can't Trace Entity Origins"

**Traditional approach**: No provenance tracking
**EdgeQuake solution**: Source ID accumulation

- source_ids: ["doc1_chunk5", "doc2_chunk3", "doc3_chunk8"]
- Full history of where entity was mentioned

### Pain Point 4: "Descriptions Get Too Long"

**Traditional approach**: Unlimited growth
**EdgeQuake solution**: Smart truncation

- Max 512 tokens per description
- Truncate at sentence boundaries
- Optional LLM summarization

## Article Angles by Platform

### Medium (Deep Technical Dive)

- The fragmentation problem diagram
- Normalization algorithm walkthrough
- Merge strategy comparison
- Production metrics

### LinkedIn (Business Value)

- "40% of entities are duplicates"
- Cost of fragmented knowledge graphs
- Data quality at scale

### X.com (Thread)

- Visual examples of normalization
- Before/after deduplication
- Real metrics

### HackerNews

- Implementation details
- Normalization edge cases
- LLM vs rule-based dedup

### Reddit (r/MachineLearning)

- Entity resolution techniques
- Graph deduplication strategies

### Substack (Newsletter)

- "The invisible duplicates destroying your RAG"
- Story-driven with practical impact
