# Iteration 08: Orient - Query Engine Analysis

## Target Audience Analysis

### Technical Leaders (CTO/VP Engineering)

**WHY they care**: Query strategy directly impacts:

- User experience (answer quality)
- Infrastructure costs (LLM token usage)
- System complexity (one engine vs multiple)

**Message focus**: Single engine handles all query types, configurable per use case

### ML/AI Engineers

**WHY they care**:

- Retrieval quality determines RAG output quality
- Wrong retrieval strategy = hallucinated answers
- Debugging retrieval is harder than debugging generation

**Message focus**: Multi-strategy retrieval with clear tradeoffs

### Data Scientists

**WHY they care**:

- Knowledge graph utilization for complex queries
- Entity relationship traversal for insights
- Balancing precision vs recall

**Message focus**: LightRAG algorithm, graph-enhanced retrieval

## Key Pain Points Query Engine Solves

### Pain Point 1: "One Retrieval Strategy Doesn't Fit All"

**Traditional approach**: Same vector search for every query
**EdgeQuake solution**: 5 modes optimized for different question types

- Naive: "What is X?" (simple lookup)
- Local: "How does A relate to B?" (entity-centric)
- Global: "What are the main themes?" (community-based)

### Pain Point 2: "No Context About WHY an Answer Was Retrieved"

**Traditional approach**: Black box similarity scores
**EdgeQuake solution**: Explicit retrieval path

- Keywords extracted from query
- Entities matched in graph
- Relationships traversed
- Sources cited

### Pain Point 3: "Token Budget Overflows"

**Traditional approach**: Stuff context until it fails
**EdgeQuake solution**: Smart token budgeting

- 4000 token default budget
- Graph context prioritized (pre-summarized)
- Truncation preserves most relevant content

### Pain Point 4: "Keyword Extraction Is Expensive"

**Traditional approach**: Extract keywords every query
**EdgeQuake solution**: 24-hour keyword cache

- Same query → cached keywords
- Similar queries benefit from cache
- 10x cost reduction on repeat queries

## Article Angles by Platform

### Medium (Deep Technical Dive)

- Full architecture diagram
- LightRAG algorithm explanation
- Mode comparison with examples
- Performance benchmarks

### LinkedIn (Business Value)

- "5 ways to retrieve context, one engine"
- Cost savings from intelligent mode selection
- Answer quality improvements

### X.com (Thread)

- Visual mode comparison
- Real-world query examples per mode
- Performance metrics

### HackerNews

- Implementation details
- Algorithm tradeoffs
- LightRAG paper discussion

### Reddit (r/MachineLearning)

- Keyword extraction approach
- Graph vs vector retrieval debate
- Evaluation methodology

### Substack (Newsletter)

- "Why your RAG answers suck"
- Story-driven mode selection
- Practical guidelines
