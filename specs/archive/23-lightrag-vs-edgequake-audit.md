# UX/UI Audit Prompt (Improved & Precision-Focused)

You are a **senior UX/UI designer / GenAI and product design auditor** specializing in **slick, modern interfaces** and Knowledge Graph.

Your task is to perform a **deep comparaison audit** of the EdgeQuake implemenatation of the Knowledge Graph UI against the Lightrag implementation of the Knowledge Graph UI.

The UX code for Lightrag is located at: lightrag_webui/
The UX code for Edgequake is located at: edgequake_webui/

You must evaluate the product as code investigator and **as a real user**, using **#playwright** to navigate the application and capture **evidence-based findings**.

## Audit Scope

- Focus on features related to Knowledge Graph visualization, interaction, and usability.
- Evaluate responsiveness, accessibility, and overall user experience.
- Identify discrepancies, bugs, and areas for improvement.
- Propose actionable recommendations for enhancing the UX/UI of EdgeQuake based on good ideas from Lightrag.
- Compare how Lightrag query the nodes and edges versus Edgequake.
- Compare how Lightrag visualize the graph versus Edgequake.
- Compare how Lightrag allow user to interact with the graph versus Edgequake.
- Compare how Lightrag allow user to filter and search the graph versus Edgequake.
- Compare how the filtering and searching impact the performance of the graph visualization in both Lightrag and Edgequake. (server side versus client side)
- Compare how Lightrag handle large graph datasets versus Edgequake.
- Suggest optimizations for graph rendering and interaction in Edgequake based on Lightrag's approach or other best practices.
- Compare how Lightrag handle graph layout and design versus Edgequake.
- Compare how Lightrag handle graph updates and real-time changes versus Edgequake.
- Compare how Lightrag handle user onboarding and tutorials for graph features versus Edgequake.
- Compare how Lightrag handle user feedback and error handling in graph features versus Edgequake.
- How to make the performance of Edgequake graph SOTA compared to Lightrag graph performance.

## Quality Checklist for Auditor

Before submitting, verify:

- [ ] Every screen has been reviewed at minimum 3 breakpoints (mobile, tablet, desktop)
- [ ] Every panel has been tested for collapsibility
- [ ] Every scrollable area has been identified and evaluated
- [ ] Screenshots are embedded for all findings
- [ ] All recommendations include specific measurements (px, rem, ms)
- [ ] Acceptance criteria are verifiable by design and engineering
- [ ] Design tokens are complete and internally consistent
- [ ] Roadmap is prioritized by impact and effort
- [ ] All markdown files are properly formatted and linked

---

## Additional Requirements

- Write everything in **Markdown**
- Embed screenshots where relevant (use descriptive alt text)
- Cross-reference any existing UX improvements already documented
- Be **opinionated, precise, and implementation-ready**
- Optimize for **clarity, consistency, slickness, and long-term scalability**
- Optimize for **performance, responsiveness, and accessibility**
- Highlight **best practices** observed in Lightrag implementation
- Identify **critical bugs or UX blockers** in EdgeQuake implementation
- Provide **detailed, step-by-step improvement plan** with prioritization
- Flag any pattern that deviates from "slick interface" standards
- Propose motion/animation enhancements where interactions feel static or abrupt

Write audit in ./audit_lighrag_vs_edgequake/ as specified above in several markdown files.

## Proposed Plan

Execute the audit in the following systematic phases:

**Phase 1: Environment Setup & Initial Investigation** (Est. 30-45 min)

- Set up EdgeQuake application at edgequake_webui/
- Start EdgeQuake services using `make dev` or equivalent commands
- Configure Playwright for automated UI capture and testing on EdgeQuake only
- Prepare LightRag codebase at lightrag_webui/ for code analysis (no Playwright)
- Create audit output directory: `./audit_lightrag_vs_edgequake/`
- Initialize `plan.md` with detailed task breakdown
- Initialize `scratchpad.md` for ongoing observations

**Phase 2: Code Architecture Analysis** (Est. 1-2 hours)

- Map component structure for both Lightrag (code review) and EdgeQuake (code review + Playwright)
- Identify graph visualization libraries used in each (D3.js, Cytoscape, Force-Graph, etc.)
- Document query mechanisms (REST API endpoints, GraphQL, WebSocket)
- Analyze state management patterns (React Context, Redux, Zustand)
- Compare data fetching strategies (client-side vs server-side filtering)
- Document performance optimization techniques (pagination, lazy loading, virtualization)

**Phase 3: Visual & Interaction Audit on EdgeQuake** (Est. 2-3 hours)

- Use Playwright to capture screenshots of all graph-related screens at 3 breakpoints (375px, 768px, 1440px)
- Test EdgeQuake graph interactions: zoom, pan, node selection, edge highlighting
- Document layout algorithms and their configurability
- Evaluate animation smoothness and transition timing
- Test EdgeQuake filtering UI: search, facets, node/edge type selection
- Measure EdgeQuake performance metrics: initial load time, interaction responsiveness, large dataset handling
- Document EdgeQuake accessibility features: keyboard navigation, screen reader support, color contrast
- Review LightRag implementations through code inspection to understand patterns and best practices

**Phase 4: Feature Parity & Gap Analysis** (Est. 1-2 hours)

- Create comparison matrix of all graph features
- Identify Lightrag features missing in EdgeQuake (from code review)
- Identify EdgeQuake features missing in Lightrag (from code review)
- Document unique approaches or innovative patterns from each implementation
- Prioritize features by user impact and implementation complexity

**Phase 5: Performance Benchmarking on EdgeQuake** (Est. 1 hour)

- Use Playwright to test EdgeQuake with small datasets (10-50 nodes)
- Test with medium datasets (100-500 nodes)
- Test with large datasets (1000+ nodes)
- Measure render time, interaction latency, memory usage
- Analyze LightRag performance approaches through code inspection
- Compare strategies for client-side vs server-side filtering

**Phase 6: Synthesis & Recommendations** (Est. 2-3 hours)

- Write detailed findings documents organized by category
- Create prioritized improvement roadmap with effort estimates
- Propose design tokens and component patterns for EdgeQuake improvements
- Document quick wins vs long-term strategic improvements
- Include code snippets and pseudo-code for key recommendations
- Create visual mockups or wireframes for proposed improvements (using Excalidraw or similar)

**Deliverables:**

- `plan.md` - Living document tracking progress
- `scratchpad.md` - Raw observations and evidence
- `01-executive-summary.md` - High-level findings and priorities
- `02-architecture-comparison.md` - Code structure and technical approaches
- `03-visual-interaction-audit.md` - UI/UX findings with EdgeQuake screenshots
- `04-feature-parity-analysis.md` - Comparison matrix and gap analysis
- `05-performance-report.md` - EdgeQuake benchmarks and optimization opportunities
- `06-recommendations-roadmap.md` - Prioritized action plan with acceptance criteria
- `07-design-tokens.md` - Proposed design system improvements for EdgeQuake
- `screenshots/` - Directory with EdgeQuake captured evidence

You will use a scratchpad.md document to collect your notes, observations, and screenshots as you navigate the application with Playwright. This file work as an append only log of your audit process.

You will use an plan.md document to outline your step-by-step plan for conducting the audit, including which screens to review, what specific elements to focus on, and how to structure your findings. Map files to be created in audit_ui/ to create a first draft of the plan, revise the plan as needed based on your audit progress and update as often as necessary while working through the audit. If you crash or need to restart, you can pick up where you left off by referring to this plan.md document.

You must propose an actionable improvement plan in several markdown files in that cross references your findings from the audit and the codebase where relevant. Cross all document your are auditing with the relevant code files in the codebase.


Be extremely careful with screenshots as it can saturate your session memory and you can die. Only take screenshots when absolutely necessary to illustrate a point. Use descriptive alt text for each screenshot to explain its relevance.
