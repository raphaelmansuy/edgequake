# EdgeQuake 2025 Roadmap (X.com Thread)

## Tweet 1/14

EdgeQuake 2025 Roadmap 🗺️

Where we're headed, what we're building, and how to get involved.

Open source projects die in silence. Roadmaps prove we're building for the long term.

Here's the plan 🧵

## Tweet 2/14

Current State:

• 11 Rust crates
• 6 query modes
• PDF, TXT, MD processing
• OpenAI + Ollama providers
• 100+ React components
• <200ms query latency
• 1000+ concurrent users

This is production-ready. But we're just getting started.

## Tweet 3/14

Q1 2025: Foundation

First quarter focuses on expanding what's possible.

• Anthropic Claude integration
• Python SDK
• CLI tool
• OpenTelemetry observability

Let's break these down ↓

## Tweet 4/14

Anthropic Claude Integration:

Claude is the #2 most requested LLM provider.

• Native adapter (not Ollama emulation)
• Claude 3 Opus, Sonnet, Haiku support
• 200K context window
• Cost tracking included

200K tokens = entire books in one call.

## Tweet 5/14

Python SDK:

70% of the ML community works in Python.

```python
from edgequake import EdgeQuake
eq = EdgeQuake(api_url="http://localhost:8080")
eq.ingest("*.pdf")
response = eq.query("What are the main findings?")
```

Pythonic. Typed. Async support.

## Tweet 6/14

Q2 2025: Enterprise

Enterprise features are adoption blockers. We're removing them.

• SSO (OIDC/SAML)
• RBAC (Role-Based Access Control)
• Document format expansion
• Audit export

## Tweet 7/14

SSO Integration:

Enterprises don't adopt tools with separate credentials.

• Okta, Auth0, Azure AD, Google
• SAML 2.0 support
• Group-to-role mapping
• Audit logging of auth events

No more "create another account."

## Tweet 8/14

RBAC:

Not everyone should access everything.

Roles:
├── Admin: Full access
├── Editor: Upload + query, no settings
├── Viewer: Query only
└── Custom: Define your own

Granular permissions at workspace level.

## Tweet 9/14

Document Formats:

PDF is common, but enterprises have:
• DOCX, XLSX, PPTX everywhere
• HTML pages
• Email archives

Q2 adds Microsoft Office and email support.

## Tweet 10/14

Q3-Q4 2025: Vision

The second half is about pushing boundaries:

• AI Agents with tool use
• Multi-hop reasoning
• Graph embeddings
• LangChain/LlamaIndex SDKs

This is where Graph-RAG gets interesting.

## Tweet 11/14

AI Agents:

RAG is just retrieval. The future is action.

User: "Find contracts expiring this quarter and draft renewal emails"

Agent:

1. Queries knowledge graph
2. Filters by date
3. Drafts personalized emails
4. Returns for approval

From search to action.

## Tweet 12/14

Contributing:

EdgeQuake is open source. Areas where you can help:

Beginner: Documentation, tests, i18n
Intermediate: LLM providers, document parsers, CLI
Advanced: Graph embeddings, AI agents, storage backends

Start: github.com/raphaelmansuy/edgequake/issues

## Tweet 13/14

Community Input:

Roadmaps are living documents. What should we prioritize?

• More LLM providers? Which ones?
• More document formats? Which ones?
• Enterprise features? What's missing?

Tell us in GitHub Discussions or reply here.

## Tweet 14/14

The Vision:

2024: Production-ready Graph-RAG
2025: Enterprise features, SDKs, AI agents
2026: The standard platform for document intelligence

We're not building a tool. We're building a platform.

🔗 github.com/raphaelmansuy/edgequake

Join us.
