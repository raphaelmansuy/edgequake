# LinkedIn Post — EdgeQuake v0.4.0

**Title:** Why your RAG pipeline struggles with PDFs — and how we fixed it with LLM Vision

---

Most RAG pipelines fail on PDFs in the same predictable way.

Text extraction works fine for clean, born-digital documents. Then you hit a scanned invoice, a multi-column research paper, a financial report with merged table cells — and the extracted text is either empty, scrambled, or structurally broken. Garbage in, garbage out. The entire knowledge graph downstream inherits that corruption.

We felt this problem firsthand while building EdgeQuake, an open-source Graph-RAG framework in Rust. Standard pdfium text extraction handled simple documents well. But production document sets are never simple.

So we asked a different question: **what if the PDF page were read the way a human reads it?**

---

**Introducing EdgeQuake v0.4.0: PDF → LLM Vision Pipeline**

Instead of extracting character codes, we now render each PDF page to a high-resolution image and send it to a multimodal LLM. The model reads the page as a human would — interpreting layout, reconstructing tables, understanding multi-column text, and handling scanned documents that have no extractable text at all.

What shipped in this release:

→ **Vision-based extraction** — GPT-4o, Claude, Gemini Vision, or any OpenAI-compatible vision model reads page images directly

→ **Handles what text extraction can't** — scanned PDFs, mixed layouts, handwritten annotations, complex tables with merged cells

→ **Zero-config pdfium** — the pdfium binary is now embedded in the package; no `PDFIUM_DYNAMIC_LIB_PATH` environment variable, no manual downloads, no CI headaches

→ **Opt-in, cost-controlled** — vision mode is disabled by default; enable per-request with `enable_vision=true` so you only pay for LLM calls on documents that need it

→ **Graceful fallback** — if the vision model is unavailable or fails, extraction automatically falls back to pdfium text mode; nothing breaks silently

→ **Full traceability** — each extracted block carries an `extraction_method` field (`vision`, `text`, or `ocr`) so you can audit exactly how every piece of content was produced

→ **Real-time progress** — live extraction progress surfaced in the WebUI via SSE streaming

The result: documents that used to produce garbled or empty content now produce clean, structured Markdown — ready for entity extraction, graph construction, and semantic search.

---

If you're building knowledge systems on top of documents, the quality of your PDF extraction is the foundation everything else sits on. We've been obsessing over this for months and this release feels like the step-change we needed.

EdgeQuake is fully open-source (Apache 2.0). The PDF pipeline works with any OpenAI-compatible provider — cloud or local (Ollama with gemma3 vision works well for air-gapped environments).

🔗 github.com/raphaelmansuy/edgequake

Happy to answer questions about the implementation — the interaction between Rust async, pdfium rendering, and vision LLM streaming was genuinely interesting to get right.

#RAG #LLM #AI #OpenSource #Rust #PDF #KnowledgeGraph #DocumentProcessing #GenAI

---
*~2,450 characters — within LinkedIn 3,000 character limit*
