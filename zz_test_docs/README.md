# Test Documents Directory Structure

This directory contains test PDFs and their corresponding gold standard markdown files for validating PDF-to-Markdown conversion quality.

## Directory Organization

```
zz_test_docs/
├── academic_papers/          # Research papers and academic documents
│   ├── *.pdf
│   └── *.pymupdf.gold.md    # PyMuPDF4LLM gold standard outputs
├── technical_docs/           # Technical documentation and guides
│   ├── *.pdf
│   └── *.pymupdf.gold.md
├── manuals/                  # Product manuals and user guides
│   ├── *.pdf
│   └── *.pymupdf.gold.md
├── presentations/            # Presentation slides and pitch decks
│   ├── *.pdf
│   └── *.pymupdf.gold.md
├── reference_materials/      # Reference docs, conventions, guides
│   ├── *.pdf
│   └── *.pymupdf.gold.md
└── generated_output/         # Previously generated outputs for testing
    └── *.md
```

## Document Categories

### Academic Papers (8 documents)
- `agentfail_2601.22984v1.pdf` - Agent paper with complex structure
- `hotmess_2601.23045v1.pdf` - Complex multi-column document
- `kvzap_2601.07891v1.pdf` - Technical research paper
- `lighrag_2410.05779v3.pdf` - LightRAG paper
- `paper_banana_2601.23265v1.pdf` - Research paper
- `stackplanner_2601.05890v1.pdf` - Technical paper
- `Qwen.pdf` - Qwen model documentation
- `The AI Hippocampus_ How Far are We From Human Memory_.pdf` - AI research paper

### Technical Documentation (3 documents)
- `AgenticPlatformReference Architecture.pdf` - Platform architecture doc
- `AI_Services__Elitizon.pdf` - Service documentation
- `Apple-Sandbox-Guide-v1.0.pdf` - Apple sandbox security guide

### Manuals (5 documents - Renault car models)
- `m_ebro_renault_clio_bja_fr_juillet_2025.pdf`
- `m_megane_etech_bcb_fr_mai_2025.pdf`
- `m_mobile_grand_kangoo_kfk_fr_avril_2025.pdf`
- `m_renault_austral_hhn_fr_mai_2025.pdf`
- `m_renault_symbioz_djb_fr_mai_2025.pdf`

### Presentations (1 document)
- `ITSalesBooster EN Presentation.pdf` - Sales presentation

### Reference Materials (5 documents)
- `001-BEYONG-TRANFORMER-OUTLINE-V1_1.pdf` - Transformer outline
- `CCN_Services_Domicile.pdf` - French service convention (50 pages)
- `SEAL_U_DM-i-0225-FR-V5.pdf` - Seal document (French)
- `Scottish SMEs Delegation - AI Learning Expedition to France - February 2026.pdf`
- `national-capitals.pdf` - National capitals reference

## Gold Standard Files

Each PDF directory contains `.pymupdf.gold.md` files generated using PyMuPDF4LLM. These serve as:
- **Reference outputs** for validation testing
- **Ground truth** for PDF-to-Markdown conversion quality metrics
- **Baseline comparisons** for edgequake extraction improvements

Generated with: `pymupdf4llm.to_markdown()`

## Usage Examples

### Running extraction tests
```bash
# Extract a single PDF with edgequake
cargo test --test pdf_extraction -- zz_test_docs/academic_papers/lighrag_2410.05779v3.pdf

# Compare with gold standard
diff <(edgequake extract zz_test_docs/academic_papers/lighrag_2410.05779v3.pdf) \
     zz_test_docs/academic_papers/lighrag_2410.05779v3.pymupdf.gold.md
```

### Validating all documents
```bash
python3 .github/skills/pdf-markdown-validator/scripts/validate.py \
  --pdf-dir zz_test_docs \
  --gold-dir zz_test_docs \
  --verbose
```

## Metrics

- **Total PDFs**: 22
- **Total size**: ~110 MB (PDFs)
- **Generated markdown**: ~1.2 MB
- **Generation tool**: PyMuPDF4LLM (pymupdf4llm)
- **Generation date**: 2026-02-07

## Notes

- Language diversity: English (majority), French (manuals, some references)
- Document types: Research papers, technical docs, manuals, presentations, references
- Complexity levels: Simple (10-50 pages) to complex (1000+ page manuals)
- Useful for testing: Multi-language, diverse layouts, tables, images, multi-column text
