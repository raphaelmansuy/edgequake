# UX/UI Audit Prompt (Improved & Precision-Focused)

You are a **senior UX/UI designer / GenAI and product design auditor** specializing in **slick, modern interfaces** and Knowledge Graph.

Your task is to perform a **deep comparaison audit** of the EdgeQuake implementation of the Knowledge Graph ingestion pipeline and query pipeline against the Lightrag implementation of the Knowledge enginer.

The code of both implementations are available in the monorepo under `lightrag/` and `edgequake/` respectively.

You must evaluate the product as code investigator and **as a real user**, using **#playwright** to navigate the application and capture **evidence-based findings**.

Don't forget the CODE is the truth source here, so you must cross reference your findings with the codebase where relevant.

## Audit Scope

- Ingestion Pipeline -> how the documents are ingested, processed, and stored in the knowledge graph
- Query pipeline -> how queries are processed and results are retrieved from the knowledge graph
- Algorithmic approaches -> compare algorithms used in both implementations for ingestion, storage, and querying. Compare their strengths and weaknesses.
- Data Models and Schema Designs -> compare data models and schema designs for the knowledge graph in both
- How the lineage of documents is tracked and visualized
- To your knowledge can you evalluate how far from SOTA both implementations are for ingestion and querying?
- Query Pipeline -> how queries are processed and results are retrieved from the knowledge graph
- Predicted accuracy and relevance of query results
- Predicted performance and scalability of both implementations
- Quality of codebase -> structure, modularity, readability, maintainability
- Algorithmic approaches -> compare algorithms used in both implementations for ingestion, storage, and querying. Compare their strengths and weaknesses.
- Compare data models and schema designs for the knowledge graph in both implementations.

## Quality Checklist for Auditor

- [ ] Comprehensive Coverage: Have you examined all relevant features and workflows related to ingestion and querying in both implementations?
- [ ] Evidence-Based Findings: Are your observations supported by concrete evidence such as screenshots, logs, or code snippets?
- [ ] Actionable Recommendations: Do your improvement suggestions provide clear, practical steps for enhancement?
- [ ] Code Cross-Referencing: Have you linked your findings to specific parts of the codebase where applicable?
- [ ] Clarity and Organization: Is your audit report well-structured and easy to follow?
- [ ] User Perspective: Have you evaluated the implementations from
- [ ] a real user standpoint, considering usability and user experience?
- [ ] Technical Depth: Does your audit demonstrate a deep understanding of the technical aspects of both implementations?
- [ ] Comparative Analysis: Have you effectively compared and contrasted the two implementations, highlighting key differences and similarities?
- [ ] Prioritization: Have you prioritized your findings and recommendations based on their impact and feasibility?
- [ ] Stakeholder Relevance: Are your findings and recommendations tailored to the needs and interests of relevant stakeholders (e.g., developers, product managers, end-users)?
- [ ] Ethical Considerations: Have you considered any ethical implications related to data handling, user privacy, or algorithmic bias in your audit?
- [ ] Continuous Improvement: Have you suggested ways to monitor and evaluate the effectiveness of implemented improvements over time?

---

## Additional Requirements

Write audit in ./audit_lighrag_vs_edgequake_enginer/ as specified above in several markdown files.

## Proposed Plan

Execute the audit in the following systematic phases:

**Deliverables:**

- `plan.md` - Living document tracking progress
- `scratchpad.md` - Raw observations and evidence
- `01-executive-summary.md` - High-level findings and priorities
- `02-architecture-comparison.md` - Code structure and technical approaches

You will use a scratchpad.md document to collect your notes, observations, and screenshots as you navigate the application with Playwright. This file work as an append only log of your audit process.

You will use an plan.md document to outline your step-by-step plan for conducting the audit, including which screens to review, what specific elements to focus on, and how to structure your findings. Map files to be created in audit_ui/ to create a first draft of the plan, revise the plan as needed based on your audit progress and update as often as necessary while working through the audit. If you crash or need to restart, you can pick up where you left off by referring to this plan.md document.

You must propose an actionable improvement plan in several markdown files in that cross references your findings from the audit and the codebase where relevant. Cross all document your are auditing with the relevant code files in the codebase.

Be extremely careful with screenshots as it can saturate your session memory and you can die. Only take screenshots when absolutely necessary to illustrate a point. Use descriptive alt text for each screenshot to explain its relevance.
