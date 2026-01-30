# Mission: EdgeQuake Promotional Article Series

## Task

Your mission is to write a series of at least 30 optimized promotional articles for EdgeQuake, a production-ready Graph-RAG framework in Rust. Each article must be published-ready for Medium.com (long-form), LinkedIn (<3000 characters), and X.com (long-form thread).

Write also hackernews posts for each article after completing the Medium, LinkedIn, and X versions.

Write also reddit posts for each article after completing the Medium, LinkedIn, and X versions. (besure to follow subreddit rules, and avoid self-promotion rules, focus on value-add content).

Write also for substack for each article after completing the Medium, LinkedIn, and X versions. Ensure the tone is more personal and newsletter-style.

You cite research papers, benchmarks, and code snippets from the EdgeQuake codebase to validate claims. You thanks research authors where relevant.

## Context

- **Location**: `./articles/` subfolder, organized as `001_subject_name/`, `002_subject_name/`, etc.
- **Product**: EdgeQuake - Advanced Retrieval-Augmented Generation (RAG) framework using graph-based knowledge representation
- **Target Audience**: CTOs, VPs of Engineering, ML Engineers, Data Scientists, DevOps engineers, and technical decision-makers
- **Goal**: Educate and generate interest in EdgeQuake's unique approach to RAG

---

## Article Topics (Minimum 15)

1. **001_why_classic_rag_fails** - Why Classic RAG Doesn't Work and Why GraphRAG Solves This
2. **002_edgequake_approach** - The EdgeQuake Approach: Graph-First RAG
3. **003_entity_extraction_deep_dive** - How EdgeQuake Extracts Knowledge from Documents
4. **004_graph_storage_architecture** - PostgreSQL AGE: The Graph Database Powering EdgeQuake
5. **005_rust_performance** - Why Rust for RAG: Performance That Matters
6. **006_llm_provider_abstraction** - LLM Agnostic Design: OpenAI, Ollama, and Beyond
7. **007_pipeline_architecture** - The Document Processing Pipeline Explained
8. **008_query_engine** - Query Engine: From Natural Language to Graph Traversal
9. **009_deduplication_normalization** - Entity Deduplication: From 40+ Entities to Clean Knowledge
10. **010_real_world_use_cases** - Real-World Use Cases: Legal, Healthcare, Finance
11. **011_cost_optimization** - Cost Optimization: $0.0014 per Document Processing
12. **012_production_deployment** - Production Deployment: From Dev to Scale
13. **013_comparison_lightrag** - EdgeQuake vs LightRAG: A Technical Comparison
14. **014_webui_experience** - The EdgeQuake WebUI: From Upload to Insight
15. **015_future_roadmap** - The Future of Graph-RAG: EdgeQuake Roadmap

---

## Article Structure Requirements

### For Each Subject Folder:

```
articles/
├── 001_why_classic_rag_fails/
│   ├── medium.md      # 1500-3000 words, SEO optimized
│   ├── linkedin.md    # <3000 characters, hook-driven
│   └── xcom.md        # Thread-optimized, 10-15 tweets
├── 002_edgequake_approach/
│   └── ...
```

### Content Principles:

1. **Start with WHY** (Simon Sinek): Lead with the problem and why it matters
2. **Feynman Technique**: Explain complex concepts simply with analogies
3. **Visual ASCII Diagrams**: Architecture flows, data pipelines, comparisons
4. **Business Value**: Real ROI, cost savings, efficiency gains
5. **Technical Depth**: Code snippets, performance metrics, benchmarks
6. **Call to Action**: Clear next steps for readers

### Quality Checklist Per Article:

- [ ] Starts with a compelling WHY
- [ ] Contains at least 2 ASCII diagrams
- [ ] Includes real metrics/benchmarks
- [ ] Has code examples where relevant
- [ ] Ends with clear CTA
- [ ] Proofread for clarity and flow
- [ ] Optimized for platform (Medium SEO, LinkedIn engagement, X virality)

---

## Process: OODA Loop (50 iterations minimum)

Execute iterative OODA cycles. Each iteration produces 4 files:

**You Must absolutely read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

Mission file: `./specs/006-write-articles.md`

You Must always produce the 4 files per iteration, as shown below:

```
specs/006-write-articles/ooda_loop/
├── iteration_01/
│   ├── observe.md   # Data gathered: code, business rules, workflows
│   ├── orient.md    # Analysis of findings vs. current docs
│   ├── decide.md    # Prioritized action plan
│   └── act.md       # Changes made, with file:line references + commit hashes
├── iteration_02/
│   └── observe.md
│   └── orient.md
│   └── decide.md
│   └── act.md
├── iteration_03/
│   └── ...
└── summary.md       # Cross-iteration insights
```

### Per-Iteration Requirements

| Step        | Output                                                     |
| ----------- | ---------------------------------------------------------- |
| **Observe** | Code analysis, feature inventory, dependency mapping       |
| **Orient**  | Gap analysis, documentation quality assessment             |
| **Decide**  | Specific changes prioritized by signal value               |
| **Act**     | Implementation with commit (`OODA-XX: <decision summary>`) |

### Constraints

1. **Re-read mission** every iteration: mission file `./specs/006-write-articles.md`
2. **Continue** from existing iterations—never restart
3. **Reference** real code: file paths, line numbers, commit SHAs
4. **Use ASCII diagrams** for architecture/flow visualization
5. **Verify facts** against actual codebase - never assume
6. **Research** current GraphRAG landscape for accurate comparisons
7. **Optimize** each article for its target platform
8. **Self-critique** brutally and refine before finalizing

**YOU Must Read your mission every iteration! It is vital to avoid alignment drift. You can forget previous iterations, but never forget your mission.**

You must always map the territory you are documenting. Never make assumptions about code structure or function. Always verify against the actual codebase.

If you don't know, make a search on the Web.

Always use First Principle Thinking as your north star.

### Deliverables

- [ ] 15+ article folders in `./articles/`
- [ ] Each folder contains: `medium.md`, `linkedin.md`, `xcom.md`
- [ ] All articles fact-checked against codebase
- [ ] ASCII diagrams embedded where relevant
- [ ] Summary document with cross-article insights

---

## ⚠️ CRITICAL SAFETY MANDATE ⚠️

**YOU MUST RE-READ THIS ENTIRE MISSION FILE AT THE START OF EVERY OODA ITERATION.**

Failure to re-read causes alignment drift → catastrophic safety issues → user frustration → system unreliability.
