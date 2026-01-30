# Why Roadmaps Matter: Building EdgeQuake for the Long Term

_On transparency, community, and building something that lasts_

---

## The Question Everyone Asks

When someone evaluates an open source project, they're really asking one question:

_"Will this still exist in two years?"_

It's a valid concern. Open source graveyards are full of ambitious projects that died quietly. No announcements, just... silence. Last commit six months ago. Issues unanswered. Stars collecting dust.

I don't want EdgeQuake to become that. So I'm publishing the roadmap.

---

## The Risk of Roadmaps

Let me be honest: roadmaps are dangerous.

They create expectations. They become promises. And software development is messy—priorities shift, discoveries change direction, life happens.

I've seen projects burned by roadmaps. They promise features, miss deadlines, and the community loses trust. Better to stay silent than to promise and fail, right?

I don't think so.

Silence creates a different kind of distrust. Users assume the worst. Contributors don't know where to help. The project feels abandoned even if it isn't.

So here's my approach: **publish the roadmap, but be honest about uncertainty**.

These are plans, not promises. Priorities may shift based on community feedback, technical discoveries, or resource constraints. The direction is real; the timeline is aspirational.

With that caveat, here's where EdgeQuake is headed.

---

## The 2025 Vision

EdgeQuake is a Graph-RAG framework—it builds knowledge graphs from documents and uses graph traversal combined with vector search to answer questions. It's production-ready today, but "production-ready" is just the starting line.

The vision for 2025 has three themes:

### 1. Meet Users Where They Are

**Python SDK**: Most ML engineers work in Python. The Rust backend is great for performance, but Pythonistas need a native interface. Q1 2025.

**CLI Tool**: Developers want `eq ingest *.pdf && eq query "summarize"` from their terminal. Unix philosophy, composable commands. Q1 2025.

**LangChain Integration**: The ecosystem matters. If you're already using LangChain, EdgeQuake should slot in as a retriever. Q3-Q4 2025.

### 2. Enterprise Readiness

**SSO (OIDC/SAML)**: Enterprises don't adopt tools with separate credentials. Okta, Auth0, Azure AD, Google—integrate with what they already use. Q2 2025.

**RBAC**: Not everyone should see everything. Admin, Editor, Viewer roles—plus custom definitions for complex organizations. Q2 2025.

**Audit Export**: Compliance teams need audit trails in their SIEM. JSON export, Splunk connectors, retention policies. Q2 2025.

These aren't exciting features. They're table stakes for enterprise adoption. We're building them because they matter.

### 3. Pushing Boundaries

**AI Agents**: RAG is just retrieval. The future is agents that take action—find contracts, draft emails, schedule meetings. Multi-turn conversations with memory and tool use. Q3-Q4 2025.

**Multi-hop Reasoning**: "Who manages the person who wrote the API spec?" requires following chains of relationships—not just retrieving documents. Q3-Q4 2025.

**Graph Embeddings**: Node2Vec and GraphSAGE enable semantic similarity at the graph level. Combine with vector search for hybrid retrieval. Q3-Q4 2025.

These are the features that excite me. They're also the hardest to build.

---

## What I've Learned About Building Open Source

EdgeQuake isn't my first open source project. Here's what I've learned about building things that last:

**1. Consistency beats intensity.**

Shipping small improvements regularly matters more than heroic pushes followed by silence. The roadmap is a reminder to keep moving.

**2. Community shapes direction.**

The features I'm most excited about aren't always the features users need most. Roadmap transparency invites input. "We want Python SDK" is more useful than me guessing.

**3. Documentation is a feature.**

Projects die when new users can't get started. I spend nearly as much time on docs as on code. It's not glamorous, but it's essential.

**4. Acknowledge limitations.**

EdgeQuake can't do everything. Being clear about what it's for (Graph-RAG, document intelligence) and what it's not for (real-time chat, general-purpose LLM hosting) builds trust.

---

## How You Can Get Involved

The roadmap isn't just for reading—it's for participating.

**Tell me what to prioritize**. Which features matter most to you? What's blocking your adoption? GitHub Discussions is the place.

**Contribute code**. Documentation improvements are beginner-friendly. LLM provider adapters are intermediate. Graph embeddings are advanced. Pick your level.

**Use it and report issues**. The best feedback comes from real usage. What worked? What was confusing? What broke?

Open source is a collaboration. The roadmap sets direction, but the community shapes the journey.

---

## The Long Game

I'm not building EdgeQuake to sell it or to get acqui-hired. I'm building it because document intelligence matters, and I think Graph-RAG is the right approach.

The goal for 2025: make EdgeQuake the obvious choice for anyone building document-based applications. Not through marketing—through quality, features, and community.

The goal for 2026 and beyond: become the standard platform for document intelligence. From startups to enterprises. From notebooks to production.

That's the long game. Roadmaps are how we get there—one quarter at a time.

---

**Repository**: github.com/raphaelmansuy/edgequake

_The roadmap is the promise. The code is the delivery. Let's build something that lasts._
