# OODA Iteration 21: ORIENT

**Focus**: Root Documentation Files (README.md, LICENSE, CONTRIBUTING.md, CODE_OF_CONDUCT.md)
**Date**: 2026-01-29

---

## Strategic Assessment

### Mission Context

**Progress**: 20/50 iterations (40% of minimum requirement)

**Current Status**: 
- ✅ Comprehensive documentation (44+ files, 10,000+ lines)
- ✅ PDF processing deep dives complete (iterations 19-20)
- ❌ Missing critical root files blocking discoverability
- ❌ No high-signal entry point for new users

**Mission Mandate**: "Ensure we have a high signal README.md at the root of the project that links to all major documentation sections."

**Priority**: **P0 CRITICAL** - Root documentation infrastructure must be in place before continuing deeper documentation work.

---

## File Prioritization

### P0 - CRITICAL (All 4 Files This Iteration)

#### 1. README.md - **HIGHEST PRIORITY**

**Signal**: 10/10 (primary discoverability)

**Impact**:
- ✅ Users landing on GitHub immediately understand project value
- ✅ Clear navigation to all documentation sections
- ✅ Professional first impression (badges, diagrams, formatting)
- ✅ Reduces support burden (self-service documentation)

**Structure**:
1. **Hero Section** (lines 1-50)
   - Project title: "EdgeQuake - High-Performance Graph-RAG Framework"
   - Tagline: "Rust-powered knowledge graph for intelligent document retrieval"
   - Badges: Build status, License, Rust version, crates.io
   - Quick value proposition (3-4 bullet points)

2. **Why EdgeQuake?** (lines 51-100)
   - Performance benchmarks vs competitors
   - Unique features (PDF extraction, graph-based retrieval)
   - When to use EdgeQuake vs alternatives

3. **Features** (lines 101-150)
   - 🚀 High Performance (Rust + async + tokio)
   - 🕸️ Knowledge Graph (Neo4j compatible, PostgreSQL AGE)
   - 📄 Advanced PDF Processing (tables, images, metadata)
   - 🔍 5 Query Modes (naive, local, global, hybrid, mix)
   - 🌐 REST API (OpenAPI 3.0, swagger UI)
   - 🎯 React 19 Frontend (real-time streaming, interactive visualizations)

4. **Quick Start** (lines 151-250)
   - Prerequisites (Rust, Node, PostgreSQL/AGE)
   - Installation (3 commands max)
   - First document ingestion (1 code block)
   - First query (1 code block)
   - Expected output (1 example)

5. **Architecture** (lines 251-350)
   - ASCII diagram of system components
   - Backend: Rust (11 crates)
   - Frontend: React 19 + TypeScript
   - Storage: PostgreSQL AGE + in-memory
   - LLM: OpenAI + Ollama + Mock

6. **Documentation Navigation** (lines 351-500)
   - **Getting Started**: Installation, Quick Start, First Ingestion
   - **Tutorials**: First RAG App, PDF Ingestion, Multi-Tenant
   - **Deep Dives**: PDF Processing, Entity Extraction, Query Modes
   - **API Reference**: REST API, Rust API
   - **Architecture**: Overview, Crates, Data Flow
   - **Operations**: Deployment, Monitoring, Scaling
   - **Troubleshooting**: Common Issues, PDF Extraction
   - **Comparisons**: vs LightRAG, vs GraphRAG
   - **More**: FAQ, Cookbook, Security, Integrations

7. **Development** (lines 501-550)
   - Link to AGENTS.md (agent workflow)
   - Link to CONTRIBUTING.md (edgecode automation)
   - Make commands (dev, test, build)
   - Testing strategy (mock provider, real LLM)

8. **Community & Support** (lines 551-600)
   - Link to CODE_OF_CONDUCT.md
   - GitHub Discussions
   - Issue tracker
   - Twitter/X: @raphaelmansuy
   - LinkedIn: raphaelmansuy

**Length**: 500-600 lines

**Effort**: Medium (4-5 hours)

**Dependencies**: docs/ structure analysis

---

#### 2. LICENSE - **CRITICAL**

**Signal**: 9/10 (legal protection)

**Impact**:
- ✅ GitHub recognizes license (badge, clarity)
- ✅ Enterprise adoption enabled (legal review passes)
- ✅ Contributor confidence (clear IP terms)
- ✅ Open source compliance

**Content**:
```
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   [Full Apache 2.0 text - 200+ lines]

   Copyright 2024-2026 Raphaël MANSUY

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```

**Length**: ~200 lines (standard)

**Effort**: Low (copy standard text, add copyright)

**Dependencies**: None

---

#### 3. CONTRIBUTING.md - **CRITICAL**

**Signal**: 8/10 (explains edgecode automation)

**Impact**:
- ✅ Contributors understand unique development model
- ✅ Specification Driven Development (SDD) workflow documented
- ✅ Raphaël MANSUY contact info clear
- ✅ Reduces invalid pull requests

**Structure**:
1. **Introduction** (lines 1-50)
   - Welcome contributors
   - Project is 100% automated by edgecode
   - edgecode: State-of-the-art coding agent by Raphaël MANSUY
   - Current status: edgecode not public yet

2. **About edgecode** (lines 51-100)
   - SOTA coding agent framework
   - Created by Raphaël MANSUY
   - Powers EdgeQuake development
   - Features:
     - Specification Driven Development (SDD)
     - OODA loop iterations
     - Multi-file context understanding
     - Test-driven implementation
     - Documentation-first approach

3. **Raphaël MANSUY** (lines 101-150)
   - **Bio**: 
     - Developer since age 14
     - Passionate about Data Engineering, Data Science, AI
     - French Tech Board Member
     - Running startup studio in Hong Kong (elitizon.com)
     - Founder of QuantaLogic AI Platform
     - Co-Founder of AI-TUTOR
     - Technical adviser at WaveX Climate-tech
     - Author: "The Definitive Guide to Data Integration"
   - **Notable Projects**:
     - code2prompt (881 stars) - LLM context generator
     - quantalogic (461 stars) - ReAct Agent Framework
     - digital_palace (68 stars) - Personal journal
   - **Contact**:
     - LinkedIn: https://www.linkedin.com/in/raphaelmansuy/
     - Twitter/X: https://twitter.com/raphaelmansuy
     - GitHub: https://github.com/raphaelmansuy
     - Newsletter: https://exponentialai.substack.com/
     - Medium: https://medium.com/@raphael.mansuy
     - Consultation: https://topmate.io/raphael_mansuy

4. **Development Workflow** (lines 151-250)
   - **Step 1**: All changes start with specifications in `specs/`
   - **Step 2**: edgecode implements specifications via OODA loop
   - **Step 3**: Automated testing and validation
   - **Step 4**: Documentation updated automatically
   - **Example**: Iteration 19-20 (PDF processing deep dive + tutorial)

5. **Specification Driven Development (SDD)** (lines 251-350)
   - Write detailed specifications first
   - OODA loop: Observe → Orient → Decide → Act
   - Each iteration: 4 markdown files (observe.md, orient.md, decide.md, act.md)
   - Minimum 50 iterations per major feature
   - Current: 20/50 iterations complete
   - Mission file: `specs/004-documentation-mission.md`

6. **How to Contribute** (lines 351-450)
   - **Option 1**: Submit specification in `specs/` directory
   - **Option 2**: Open GitHub issue with detailed requirements
   - **Option 3**: Contact Raphaël MANSUY directly
   - **Not Accepted**: Direct code pull requests (bypassing SDD workflow)
   - **Why**: Maintains consistency and quality via edgecode automation

7. **Development Standards** (lines 451-500)
   - **Rust**: 
     - Use `cargo fmt`, `cargo clippy` before commit
     - No `unwrap()` in production code
     - Use `tracing` crate for logging
     - Follow standard Rust naming conventions
   - **TypeScript**: 
     - Two-space indentation
     - Functional React components
     - ESLint + Prettier
   - **Documentation**: 
     - Always update docs/ for user-facing changes
     - Code comments explain "why", not "what"
   - **Testing**: 
     - All features require tests
     - Mock provider for CI/CD
     - Real LLM for production validation

8. **edgecode Future** (lines 501-550)
   - edgecode will be open-sourced soon
   - Will enable community-driven development
   - Current: Private tool by Raphaël MANSUY
   - Follow Raphaël's newsletter for updates

**Length**: 500-550 lines

**Effort**: Medium (3-4 hours)

**Dependencies**: Raphaël MANSUY bio (completed via web research)

---

#### 4. CODE_OF_CONDUCT.md - **REQUIRED**

**Signal**: 7/10 (community standards)

**Impact**:
- ✅ GitHub "Community" tab complete
- ✅ Behavioral expectations clear
- ✅ Enforcement mechanism documented
- ✅ Professional appearance for enterprise adoption

**Content**: Contributor Covenant 2.1 (industry standard)

**Structure**:
1. **Our Pledge** (lines 1-50)
   - Open, welcoming, diverse, inclusive community
   - Harassment-free experience for all
   - Respect differences

2. **Our Standards** (lines 51-100)
   - **Examples of positive behavior**:
     - Empathy and kindness
     - Respectful disagreement
     - Constructive feedback
     - Community focus
   - **Examples of unacceptable behavior**:
     - Sexual language/imagery
     - Trolling, insults, personal attacks
     - Public or private harassment
     - Publishing others' private information

3. **Enforcement Responsibilities** (lines 101-150)
   - Community leaders enforce standards
   - Clarify acceptable behavior
   - Take fair corrective action
   - Remove/reject contributions violating code

4. **Scope** (lines 151-200)
   - Applies to all project spaces
   - Applies when representing project in public
   - Examples: email, social media, events

5. **Enforcement** (lines 201-250)
   - Report violations to Raphaël MANSUY
   - Contact: [Insert email]
   - Confidential review
   - Fair enforcement
   - Enforcement Guidelines (levels 1-4)

6. **Attribution** (lines 251-280)
   - Adapted from Contributor Covenant 2.1
   - https://www.contributor-covenant.org/version/2/1/code_of_conduct/

**Length**: 250-280 lines

**Effort**: Low (adapt standard template)

**Dependencies**: Raphaël MANSUY contact email

---

## Strategic Insights

### From Web Research: Raphaël MANSUY

**Professional Background**:
- **Developer**: Since age 14 (30+ years experience)
- **Expertise**: Data Engineering, Data Science, AI/LLM
- **Leadership**:
  - CTO at Elitizon Ltd (startup studio, Hong Kong)
  - Founder: QuantaLogic AI Platform
  - Co-Founder: AI-TUTOR
  - Technical Adviser: WaveX Climate-tech (London)
  - French Tech Board Member
- **Author**: "The Definitive Guide to Data Integration" (Packt)
- **Open Source**:
  - 159 GitHub repositories
  - 2,211 contributions last year
  - Notable projects: code2prompt (881★), quantalogic (461★), digital_palace (68★)
- **Community**:
  - Newsletter: Exponential AI (https://exponentialai.substack.com/)
  - Medium: AI and Data Engineering articles
  - Active on X/Twitter, LinkedIn

**Key Projects**:
1. **code2prompt** (881 stars): LLM context generator tool
2. **quantalogic** (461 stars): ReAct Agent Framework for coding agents
3. **adk_training** (70 stars): Google ADK training with 34 tutorials
4. **adk-code** (20 stars): AI coding assistant in Go

**Insight**: Raphaël is deeply invested in AI agents and LLM tooling. EdgeQuake being "100% automated by edgecode" is consistent with his work on quantalogic and other AI agent frameworks.

---

## Risk Assessment

### Risk 1: README Too Generic

**Probability**: Medium

**Impact**: High (fails mission requirement for "high signal")

**Mitigation**:
- ✅ Include concrete benchmarks (vs LightRAG, GraphRAG)
- ✅ Show unique features (PDF extraction, 5 query modes)
- ✅ Add ASCII architecture diagram
- ✅ Real code examples (not placeholders)
- ✅ Link to all 14 documentation sections

---

### Risk 2: CONTRIBUTING.md Confuses Contributors

**Probability**: Medium

**Impact**: Medium (invalid PRs, frustrated contributors)

**Mitigation**:
- ✅ Clear explanation of edgecode automation
- ✅ Simple contribution path (contact Raphaël directly)
- ✅ Examples of successful specifications (iterations 19-20)
- ✅ FAQ: "Why no direct PRs?" → "Maintains consistency via SDD"

---

### Risk 3: Missing Contact Info

**Probability**: Low

**Impact**: High (blocks contribution, enforcement)

**Mitigation**:
- ✅ Multiple contact methods (LinkedIn, Twitter, GitHub, email)
- ✅ Newsletter for updates (Exponential AI)
- ✅ Consultation booking link (Topmate)

---

### Risk 4: Documentation Link Rot

**Probability**: Low

**Impact**: Medium (broken links, user frustration)

**Mitigation**:
- ✅ Use relative paths (docs/getting-started/installation.md)
- ✅ Verify all links exist before commit
- ✅ Group by category (Getting Started, Tutorials, etc.)

---

## Content Reuse Strategy

### From edgequake/README.md (177 lines)

**Reuse**:
- ✅ Features list (6 bullet points with emojis)
- ✅ API endpoints table
- ✅ Query modes comparison table
- ✅ Project structure (11 crates)
- ✅ Quick start code examples

**Expand**:
- ✅ Add PDF processing features (iterations 19-20)
- ✅ Add frontend (edgequake_webui) section
- ✅ Add docs/ navigation (14 categories)
- ✅ Add architecture diagram (ASCII)
- ✅ Add "Why EdgeQuake?" section

**Keep Separate**:
- ✅ edgequake/README.md stays Rust-focused
- ✅ Root README.md covers entire project ecosystem

---

### From docs/README.md (Documentation Index)

**Reuse**:
- ✅ Section structure (Getting Started, Tutorials, etc.)
- ✅ File listings per section
- ✅ Brief descriptions

**Expand**:
- ✅ Add one-sentence description per major section
- ✅ Highlight new content (PDF processing, tutorials)
- ✅ Cross-reference related sections

---

## ASCII Diagram Strategy

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        EdgeQuake System                      │
└─────────────────────────────────────────────────────────────┘

┌──────────────────┐         ┌──────────────────┐
│  React 19 Frontend│◄───────►│  Axum REST API   │
│  (edgequake_webui)│         │  (edgequake-api) │
└──────────────────┘         └──────────────────┘
                                      │
                      ┌───────────────┼───────────────┐
                      ▼               ▼               ▼
            ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
            │ Document     │ │ Query        │ │ Knowledge    │
            │ Pipeline     │ │ Engine       │ │ Graph        │
            │ (ingestion)  │ │ (5 modes)    │ │ (entities)   │
            └──────────────┘ └──────────────┘ └──────────────┘
                      │               │               │
                      └───────────────┼───────────────┘
                                      ▼
                      ┌───────────────────────────────┐
                      │      Storage Layer            │
                      │  ┌──────────┐  ┌──────────┐  │
                      │  │PostgreSQL│  │ Memory   │  │
                      │  │   AGE    │  │ (Mock)   │  │
                      │  └──────────┘  └──────────┘  │
                      └───────────────────────────────┘
                                      │
                      ┌───────────────┼───────────────┐
                      ▼               ▼               ▼
            ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
            │   OpenAI     │ │   Ollama     │ │    Mock      │
            │   (gpt-4o)   │ │ (local LLM)  │ │ (testing)    │
            └──────────────┘ └──────────────┘ └──────────────┘
```

**Lines**: ~25

**Value**: Visual understanding of system components and data flow

---

### OODA Loop Diagram (for CONTRIBUTING.md)

```
Specification Driven Development (SDD) Workflow:

specs/
  └── <feature-name>.md ──────► edgecode (SOTA Agent)
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              ┌──────────┐    ┌──────────┐    ┌──────────┐
              │ OBSERVE  │───►│  ORIENT  │───►│  DECIDE  │
              │ (analyze)│    │(strategize)   │  (plan)  │
              └──────────┘    └──────────┘    └──────────┘
                                                     │
                                                     ▼
                                              ┌──────────┐
                                              │   ACT    │
                                              │(implement)
                                              └──────────┘
                                                     │
                    ┌────────────────────────────────┘
                    ▼
          ┌──────────────────┐
          │   Git Commit     │
          │  OODA-<N>        │
          └──────────────────┘
                    │
                    └────► Next Iteration (N+1)

Current: 20/50 iterations complete
```

**Lines**: ~20

**Value**: Explains edgecode automation workflow visually

---

## Implementation Sequence

### Order (Parallel Where Possible)

1. **LICENSE** (30 min)
   - Copy Apache 2.0 template
   - Add copyright notice
   - ✅ No dependencies

2. **CODE_OF_CONDUCT.md** (1 hour)
   - Copy Contributor Covenant 2.1
   - Customize contact info
   - ✅ No dependencies

3. **README.md** (4-5 hours)
   - Hero section
   - Why EdgeQuake?
   - Features (reuse + expand)
   - Quick Start (reuse + verify)
   - Architecture (ASCII diagram)
   - Documentation navigation (14 sections)
   - Development & community
   - ✅ Depends on: docs/ structure analysis

4. **CONTRIBUTING.md** (3-4 hours)
   - Introduction (edgecode automation)
   - About edgecode
   - Raphaël MANSUY bio (from web research)
   - Development workflow
   - SDD explanation
   - Contribution path
   - Standards
   - ✅ Depends on: Raphaël bio (complete)

**Total Estimated Time**: 9-11 hours

**Parallelizable**: 
- LICENSE + CODE_OF_CONDUCT.md (1.5 hours)
- README.md + CONTRIBUTING.md (7.5-9.5 hours)

**Sequential**: None (all can start after OBSERVE/ORIENT complete)

---

## Success Criteria

### Quantitative Metrics

- ✅ README.md: 500-600 lines, 20+ links, 1-2 ASCII diagrams
- ✅ LICENSE: ~200 lines, GitHub recognition
- ✅ CONTRIBUTING.md: 500-550 lines, OODA diagram, Raphaël bio
- ✅ CODE_OF_CONDUCT.md: 250-280 lines, Contributor Covenant 2.1

### Qualitative Metrics

**README.md**:
- ✅ User understands EdgeQuake value in < 30 seconds
- ✅ User finds relevant docs in < 1 minute
- ✅ Professional appearance (badges, diagrams, formatting)
- ✅ Clear differentiation from competitors

**LICENSE**:
- ✅ GitHub "About" section shows Apache 2.0
- ✅ Enterprise legal review passes

**CONTRIBUTING.md**:
- ✅ edgecode automation clearly explained
- ✅ SDD workflow documented with diagram
- ✅ Raphaël contact info complete
- ✅ Contribution path crystal clear

**CODE_OF_CONDUCT.md**:
- ✅ GitHub "Community" tab shows complete
- ✅ Enforcement process documented

---

## Strategic Recommendations

### Recommendation 1: High-Signal README

**Rationale**: Mission requires "high signal" README

**Actions**:
- ✅ Lead with concrete value props (not generic features)
- ✅ Include performance benchmarks (vs competitors)
- ✅ Show real code examples (not placeholders)
- ✅ ASCII architecture diagram (visual understanding)
- ✅ Prominent links to all docs/ sections

---

### Recommendation 2: Emphasize PDF Capabilities

**Rationale**: Iterations 19-20 added comprehensive PDF documentation

**Actions**:
- ✅ Feature: "Advanced PDF Processing (tables, images, metadata)"
- ✅ Link: docs/deep-dives/pdf-processing.md (NEW in README)
- ✅ Link: docs/tutorials/pdf-ingestion.md (NEW in README)
- ✅ Quick Start: Include PDF ingestion example

---

### Recommendation 3: Clear edgecode Explanation

**Rationale**: Unique development model may confuse contributors

**Actions**:
- ✅ Dedicated "About edgecode" section (100+ lines)
- ✅ OODA loop diagram (visual explanation)
- ✅ Real example (iterations 19-20)
- ✅ FAQ: "Why no direct PRs?"

---

### Recommendation 4: Multiple Contact Methods

**Rationale**: Reduce friction for legitimate contributors

**Actions**:
- ✅ GitHub issues (low barrier)
- ✅ LinkedIn (professional)
- ✅ Twitter/X (community)
- ✅ Email (direct)
- ✅ Newsletter (updates)
- ✅ Topmate (consultation)

---

## Next: DECIDE Phase

**Objective**: Design complete structure and content for all 4 files

**Deliverables**:
1. README.md outline (500-600 lines)
2. LICENSE content (Apache 2.0 + copyright)
3. CONTRIBUTING.md outline (500-550 lines)
4. CODE_OF_CONDUCT.md content (Contributor Covenant 2.1)

**Duration**: 1-2 hours

**Dependencies**: None (all research complete)
