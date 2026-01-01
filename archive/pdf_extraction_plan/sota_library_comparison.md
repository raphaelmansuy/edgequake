# SOTA PDF-to-Markdown Libraries Comparison

**Date:** 2025-12-31  
**Purpose:** Comprehensive analysis of state-of-the-art PDF-to-markdown libraries to guide EdgeQuake-PDF improvements

---

## Executive Summary

After extensive research, the **top 5 SOTA libraries** for PDF-to-markdown conversion are:

| Rank | Library         | Stars | Approach             | Accuracy | Speed     | License  |
| ---- | --------------- | ----- | -------------------- | -------- | --------- | -------- |
| 1    | **MinerU**      | 51.3k | Hybrid (VLM+OCR)     | 90+      | Medium    | AGPL-3.0 |
| 2    | **Docling**     | 48.5k | Multi-model pipeline | ~85      | Medium    | MIT      |
| 3    | **Marker**      | 30.7k | Deep learning models | 95.7     | 25 pg/s   | GPL-3.0  |
| 4    | **Surya**       | 19.1k | OCR + Layout models  | 97       | Very Fast | GPL-3.0  |
| 5    | **PyZerox**     | ~1k   | Vision-first LLM     | High     | Slow      | MIT      |
| 6    | **Nougat**      | 9.8k  | Neural OCR (papers)  | Moderate | Medium    | MIT      |
| 7    | **PyMuPDF4LLM** | 1.2k  | Text extraction      | Moderate | Very Fast | AGPL-3.0 |

**Recommendation:** Implement a **Marker-like hybrid approach** with optional vision mode.

---

## 1. MinerU (OpenDataLab)

**Repository:** https://github.com/opendatalab/MinerU  
**Stars:** 51,300+ | **Language:** Python | **License:** AGPL-3.0

### Overview

MinerU is from OpenDataLab (InternLM team). Newest release (v2.7.0) introduced a **hybrid backend** combining VLM and pipeline approaches.

### Key Features

- ✅ Remove headers, footers, footnotes, page numbers
- ✅ Human-readable reading order (single-column, multi-column, complex layouts)
- ✅ Preserve document structure (headings, paragraphs, lists)
- ✅ Extract images, tables, and formulas (LaTeX)
- ✅ Auto-detect scanned PDFs and enable OCR (109 languages)
- ✅ Multiple backends: `pipeline`, `vlm`, `hybrid`
- ✅ GPU/CPU/NPU/MPS acceleration

### Architecture

```
PDF → Layout Detection → Content Extraction → Reading Order → Output
         ↓                    ↓
   DocLayout-YOLO      PDF-Extract-Kit + OCR + UniMERNet (formulas)
```

### Backend Comparison

| Backend  | Accuracy | Requirements      |
| -------- | -------- | ----------------- |
| pipeline | 82+      | Pure CPU possible |
| vlm      | 90+      | 10GB+ VRAM        |
| hybrid   | 90+      | Best of both      |

### Usage

```bash
pip install uv && uv pip install -U "mineru[all]"
mineru -p input.pdf -o output/ -b hybrid-auto-engine
```

### Rust Consideration

Python-only, but uses PyTorch models that could be ported to `candle` or `burn`.

---

## 2. Docling (IBM)

**Repository:** https://github.com/docling-project/docling  
**Stars:** 48,500+ | **Language:** Python | **License:** MIT

### Overview

IBM's document understanding library, hosted by Linux Foundation AI & Data.

### Key Features

- ✅ Multiple formats: PDF, DOCX, PPTX, XLSX, HTML, WAV, MP3, VTT, images
- ✅ Advanced PDF understanding (layout, reading order, tables, formulas)
- ✅ Visual Language Model support (GraniteDocling)
- ✅ Audio support with ASR models
- ✅ MCP server for agentic applications
- ✅ LangChain, LlamaIndex, Crew AI, Haystack integrations

### New Features (v2.66)

- New layout model (Heron) for faster PDF parsing
- Structured information extraction
- Web Video Text Tracks (WebVTT) parsing

### Usage

```python
from docling.document_converter import DocumentConverter

converter = DocumentConverter()
result = converter.convert("https://arxiv.org/pdf/2408.09869")
print(result.document.export_to_markdown())
```

### Architecture

Uses a unified `DoclingDocument` representation format with multiple export options (Markdown, HTML, DocTags, JSON).

---

## 3. Marker (Datalab)

**Repository:** https://github.com/datalab-to/marker  
**Stars:** 30,700+ | **Language:** Python | **License:** GPL-3.0

### Overview

Marker is a **high-accuracy, high-speed** PDF converter. Benchmarks favorably against LlamaParse, Mathpix, and Docling.

### Key Features

- ✅ Converts PDF, PPTX, DOCX, XLSX, HTML, EPUB, images
- ✅ Tables, forms, equations, inline math, links, code blocks
- ✅ Extracts and saves images
- ✅ Removes headers/footers/artifacts
- ✅ Extensible with custom formatting
- ✅ Structured extraction given JSON schema (beta)
- ✅ **Hybrid LLM mode** for highest accuracy
- ✅ GPU, CPU, or MPS acceleration

### Benchmark Results (H100)

| Tool       | Speed (s/pg) | Accuracy | LLM Score |
| ---------- | ------------ | -------- | --------- |
| **Marker** | 2.84         | 95.67%   | 4.24      |
| LlamaParse | 23.35        | 84.24%   | 3.98      |
| Mathpix    | 6.36         | 86.43%   | 4.16      |
| Docling    | 3.70         | 86.71%   | 3.70      |

**Throughput:** 25 pages/second on H100 in batch mode (projected 122 pages/second).

### Hybrid LLM Mode

```bash
marker_single /path/to/file.pdf --use_llm --gemini_api_key YOUR_KEY
```

Hybrid mode offers **higher accuracy than marker or Gemini alone** for tables.

### Architecture

```
PDF → Extract text (heuristics + Surya OCR)
    → Detect layout + reading order (Surya)
    → Clean/format each block (heuristics + Texify)
    → [Optional] LLM enhancement
    → Combine blocks + postprocess
```

### LLM Services Supported

- Gemini (default)
- Google Vertex
- Ollama (local models)
- Claude
- OpenAI
- Azure OpenAI

### Why Marker Stands Out

1. **Highest accuracy** in benchmarks (95.67%)
2. **Fastest** among high-accuracy tools
3. **Hybrid LLM mode** combines best of both worlds
4. **Extensible architecture** (Providers, Builders, Processors, Renderers)
5. **Multiple output formats** (Markdown, JSON, HTML, Chunks)

---

## 4. Surya (Datalab)

**Repository:** https://github.com/datalab-to/surya  
**Stars:** 19,100+ | **Language:** Python | **License:** GPL-3.0

### Overview

Surya is the **underlying OCR toolkit** used by Marker. Focuses on document OCR with deep learning models.

### Capabilities

| Capability        | Performance                 |
| ----------------- | --------------------------- |
| OCR               | 90+ languages, 97% accuracy |
| Line Detection    | Any language                |
| Layout Analysis   | Table, Image, Header, etc.  |
| Reading Order     | 88% accuracy                |
| Table Recognition | Rows/columns with text      |
| LaTeX OCR         | Equations → LaTeX           |

### Layout Labels Detected

`Caption`, `Footnote`, `Formula`, `List-item`, `Page-footer`, `Page-header`, `Picture`, `Figure`, `Section-header`, `Table`, `Form`, `Table-of-contents`, `Handwriting`, `Text`, `Text-inline-math`

### Benchmarks

| Task            | Surya | Tesseract |
| --------------- | ----- | --------- |
| OCR Speed       | 0.62s | 0.45s     |
| OCR Accuracy    | 97%   | 88%       |
| Detection Speed | 47.2s | 74.5s     |

### Python Usage

```python
from surya.foundation import FoundationPredictor
from surya.recognition import RecognitionPredictor
from surya.detection import DetectionPredictor

foundation = FoundationPredictor()
recognition = RecognitionPredictor(foundation)
detection = DetectionPredictor()

predictions = recognition([image], det_predictor=detection)
```

### Rust Consideration

Models are PyTorch-based. Could potentially port to Rust using:

- `candle` (Hugging Face's Rust ML framework)
- `burn` (Native Rust deep learning)
- ONNX export → `tract` or `ort`

---

## 5. PyZerox

**Repository:** https://github.com/getomni-ai/zerox  
**Stars:** ~1,000 | **Language:** Python | **License:** MIT

### Overview

Vision-first approach: render PDF pages as images, send to vision LLM for markdown extraction.

### Architecture

```
PDF → Images (poppler/GraphicsMagick) → Vision LLM → Markdown
```

### Key Features

- ✅ Multi-provider: OpenAI, Claude, Gemini, Bedrock
- ✅ `maintain_format` for cross-page context
- ✅ Concurrent processing
- ✅ Custom system prompts

### Trade-offs

| Pros                          | Cons                         |
| ----------------------------- | ---------------------------- |
| Excellent for complex layouts | Expensive ($0.01-0.03/page)  |
| Tables work perfectly         | Slow (LLM latency)           |
| No training required          | Requires API access          |
| Works with any document type  | No embedded image extraction |

---

## 6. Nougat (Meta Research)

**Repository:** https://github.com/facebookresearch/nougat  
**Stars:** 9,800+ | **Language:** Python | **License:** MIT (code), CC-BY-NC (model)

### Overview

Neural OCR specifically for **academic documents** (arXiv, PMC papers). Built on Donut architecture.

### Best For

- Scientific papers with LaTeX math
- arXiv-style documents
- Tables in academic format

### Limitations

- Only works well with English academic papers
- Chinese, Japanese, Russian etc. **will not work**
- Limited to paper-like layouts

### Usage

```bash
pip install nougat-ocr
nougat path/to/paper.pdf -o output_dir
```

---

## 7. PyMuPDF4LLM

**Repository:** https://github.com/pymupdf/pymupdf4llm  
**Stars:** 1,200+ | **Language:** Python | **License:** AGPL-3.0

### Overview

LLM-optimized text extraction from PyMuPDF. Very fast, rule-based approach.

### Key Features

- ✅ Clean structured Markdown
- ✅ Preserves headers, lists, tables
- ✅ Multi-column support
- ✅ Image extraction option
- ✅ Supports PDF, XPS, EPUB, MOBI

### Limitations

- No deep learning models
- Relies on PDF structure (fails for scanned PDFs)
- No layout detection

### Usage

```python
import pymupdf4llm
md_text = pymupdf4llm.to_markdown("input.pdf")
```

---

## Comparative Analysis

### By Use Case

| Use Case                   | Best Library         | Alternative      |
| -------------------------- | -------------------- | ---------------- |
| Academic Papers            | Marker, Nougat       | MinerU           |
| Complex Business Documents | MinerU, Docling      | Marker + LLM     |
| Simple Text PDFs           | PyMuPDF4LLM          | EdgeQuake-PDF    |
| Scanned Documents          | Surya → Marker       | MinerU           |
| Multi-language OCR         | MinerU (109 langs)   | Surya (90 langs) |
| Tables                     | Marker + LLM         | Docling          |
| Cost-sensitive             | PyMuPDF4LLM          | Marker (no LLM)  |
| API Integration            | Docling (MCP server) | PyZerox          |
| RAG Applications           | Marker (chunks mode) | Docling          |

### By Technical Approach

| Approach          | Libraries                | Accuracy | Speed   | Cost    |
| ----------------- | ------------------------ | -------- | ------- | ------- |
| Text Extraction   | PyMuPDF4LLM, pdf_oxide   | 70-80%   | Fastest | Free    |
| Deep Learning OCR | Surya, Marker            | 90-97%   | Fast    | Free    |
| Vision LLM        | PyZerox                  | 85-95%   | Slow    | $$$     |
| Hybrid (DL + LLM) | Marker + LLM, MinerU     | 95+%     | Medium  | $$      |
| End-to-End VLM    | Docling (GraniteDocling) | 85-90%   | Medium  | Free/$$ |

---

## Recommendations for EdgeQuake-PDF

### Priority 1: Adopt Marker's Architecture Pattern

Marker's architecture is the most extensible and performant:

```rust
pub trait Provider { /* Extract raw content from PDF */ }
pub trait Builder { /* Create document blocks */ }
pub trait Processor { /* Process specific block types */ }
pub trait Renderer { /* Output markdown/json/html */ }
```

### Priority 2: Implement Layout Detection

Use similar approach to Surya for layout detection:

- Block types: Table, Figure, Header, Text, List, etc.
- Reading order detection
- Multi-column handling

### Priority 3: Hybrid LLM Mode

Like Marker's `--use_llm` flag:

```rust
pub struct ExtractionConfig {
    pub use_llm: bool,
    pub llm_provider: LlmProvider,
    pub llm_model: String,
    // ...
}
```

LLM enhancement for:

- Table formatting
- Inline math conversion
- Cross-page content merging
- Form value extraction

### Priority 4: Vision Mode for Complex Documents

For documents where text extraction fails:

```rust
pub enum ExtractionMode {
    TextBased,      // Fast, good for simple PDFs
    VisionBased,    // Accurate, expensive
    Hybrid,         // Auto-detect complexity
}
```

### Priority 5: Output Format Flexibility

Match Marker's output options:

- Markdown (GitHub-compatible)
- JSON (structured with bboxes)
- HTML
- Chunks (RAG-optimized)

---

## Implementation Roadmap for EdgeQuake-PDF

### Phase 1: Core Improvements (2-3 weeks)

- [ ] Implement block-based extraction (like Marker's schema)
- [ ] Add layout detection using heuristics
- [ ] Improve table detection and formatting
- [ ] Add reading order detection

### Phase 2: LLM Enhancement (1-2 weeks)

- [ ] Add `use_llm` option for enhanced extraction
- [ ] Implement table formatting with LLM
- [ ] Add inline math detection and LaTeX conversion
- [ ] Cross-page context handling

### Phase 3: Vision Mode (2-3 weeks)

- [ ] Add PDF-to-image rendering (pdfium-render)
- [ ] Implement vision-based extraction
- [ ] Add complexity detection for hybrid mode
- [ ] Benchmark against Marker/MinerU

### Phase 4: Output Formats (1 week)

- [ ] JSON output with block structure
- [ ] HTML output
- [ ] Chunks mode for RAG

---

## Rust Crates to Consider

| Purpose          | Crate Options                          |
| ---------------- | -------------------------------------- |
| PDF Rendering    | `pdfium-render`, `mupdf`, `pdf-render` |
| Image Processing | `image`, `imageproc`                   |
| ML Inference     | `candle`, `burn`, `tract`, `ort`       |
| Table Detection  | Custom heuristics or port Surya        |
| Layout Analysis  | Port Surya layout model                |
| OCR              | `tesseract` bindings or port Surya     |

---

## Conclusion

**Marker** represents the current SOTA for PDF-to-markdown with its:

- 95.67% accuracy (highest in benchmarks)
- 25 pages/second throughput
- Hybrid LLM mode option
- Extensible architecture

**MinerU** offers the best hybrid VLM approach with:

- 90+ accuracy with VLM backend
- 109 language OCR support
- Active development (v2.7.0 just released)

For **EdgeQuake-PDF**, the recommended path is:

1. Adopt Marker's modular architecture
2. Implement layout detection
3. Add optional LLM enhancement
4. Consider vision mode for complex documents

This would position EdgeQuake-PDF as a **Rust-native alternative** to Marker with similar capabilities.
