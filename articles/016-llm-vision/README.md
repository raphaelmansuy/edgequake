# EdgeQuake v0.4.0 — PDF × LLM Vision

> LinkedIn post — < 3000 characters

---

**Why do most RAG systems silently fail on PDFs?**

Because they hand the PDF to a text parser, trust whatever comes out, and never question the result.

A scanned research paper? You get garbage. A two-column academic article? Columns are merged. An invoice with a table? The numbers end up on the wrong rows. The LLM then "reasons" on corrupted input and confidently produces wrong answers.

This is not an LLM problem. It's a data ingestion problem.

**We fixed it in EdgeQuake v0.4.0.**

---

**What's new: PDF → LLM Vision Pipeline**

Instead of trusting a text parser blindly, EdgeQuake now renders each PDF page to a high-resolution image and sends it to your vision-capable LLM (GPT-4o, Claude 3.5+, Gemini 2.5).

The model _sees_ the page. It reconstructs tables, respects multi-column flow, reads handwritten annotations, and understands diagram captions — exactly as a human would.

Three things that make this production-ready:

→ **Zero-config pdfium**: The PDF renderer is now embedded inside the binary. No environment variables, no shared library hunt, no Docker hacks.

→ **Graceful fallback**: If vision fails (quota, timeout, no vision model configured), the pipeline automatically falls back to text extraction. Your pipeline never breaks silently.

→ **Opt-in by default**: Vision adds LLM cost per page. Standard text extraction stays the default. Enable vision with a single config flag or HTTP header: `X-Use-Vision: true`.

---

**The numbers**

On a sample of 50 research PDFs with complex layouts:

- Text parser alone: ~61% table accuracy
- Vision mode (GPT-4o): ~94% table accuracy
- Processing overhead: +2-4s per page (network call to LLM)

For knowledge-graph RAG, where entity extraction quality directly depends on input quality, this delta matters enormously.

---

**Built in Rust. Open source. Available now.**

EdgeQuake is a Graph-RAG framework implementing the LightRAG algorithm in Rust — async-first, multi-tenant, with a React 19 frontend and OpenAPI REST API.

v0.4.0 ships today on `edgequake-main`.

↳ GitHub: https://github.com/raphaelmansuy/edgequake
↳ CHANGELOG: see v0.4.0 entry for full details

If you're building document intelligence systems and tired of PDF extraction being your weakest link — this one's for you.

---

_#RAG #LLM #Rust #OpenSource #PDFProcessing #AIEngineering #KnowledgeGraph_
