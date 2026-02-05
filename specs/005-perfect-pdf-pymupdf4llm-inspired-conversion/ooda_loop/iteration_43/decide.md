# OODA-43: Decide - Phased Integration Approach

## Date: 2026-02-05

## Decision

**Create PdfiumBackend that uses pdfium for extraction but integrates with existing schema and ProcessorChain.**

---

## Approach: Bridge the Pipelines

Instead of a full refactoring, we'll create a bridge:

```
┌────────────────────────────────────────────────────────────────┐
│                    PdfiumBackend (NEW)                         │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────┐ │
│  │ PdfiumExtr.  │  →   │ TextGrouper  │  →   │ Convert to   │ │
│  │ (RawChar[])  │      │ (TextBlock[])│      │ Document IR  │ │
│  └──────────────┘      └──────────────┘      └──────────────┘ │
│                                                    │           │
│                                                    ▼           │
│                           ┌───────────────────────────────────┐│
│                           │ Existing ProcessorChain          ││
│                           │ (headers, tables, lists, etc.)   ││
│                           └───────────────────────────────────┘│
│                                                    │           │
│                                                    ▼           │
│                           ┌───────────────────────────────────┐│
│                           │ MarkdownRenderer                 ││
│                           └───────────────────────────────────┘│
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## Implementation Steps

### Step 1: Create PdfiumBackend struct

File: `src/backend/pdfium_backend.rs`

```rust
use crate::backend::{PdfBackend, PdfiumExtractor};
use crate::config::PdfConfig;
use crate::layout::{TextGrouper, GroupingParams};
use crate::schema::{Document, Page, Block, BlockType, BoundingBox};

pub struct PdfiumBackend {
    extractor: PdfiumExtractor,
    config: PdfConfig,
}

impl PdfiumBackend {
    pub fn new(config: PdfConfig) -> Result<Self, PdfError> {
        let extractor = PdfiumExtractor::new()?;
        Ok(Self { extractor, config })
    }
}
```

### Step 2: Implement PdfBackend trait

```rust
#[async_trait]
impl PdfBackend for PdfiumBackend {
    async fn extract(&self, pdf_bytes: &[u8]) -> Result<Document> {
        // 1. Get raw characters with accurate positions
        let chars = self.extractor.extract_chars_from_bytes(pdf_bytes)?;

        // 2. Group into semantic blocks using pymupdf-style algorithm
        let grouper = TextGrouper::with_params(GroupingParams::default());
        let mut blocks = grouper.group(&chars);

        // 3. Classify blocks (headers, lists, code)
        let body_size = detect_body_font_size(&blocks);
        grouper.classify_blocks(&mut blocks, body_size);

        // 4. Convert to Document IR for ProcessorChain compatibility
        self.convert_to_document(blocks)
    }
}
```

### Step 3: Convert TextBlock to schema::Block

Key conversion:

```rust
fn convert_block(text_block: &TextBlock) -> Block {
    let block_type = match text_block.block_type {
        layout::BlockType::Header(level) => BlockType::Header(level),
        layout::BlockType::Paragraph => BlockType::Text,
        layout::BlockType::ListItem => BlockType::ListItem,
        layout::BlockType::Code => BlockType::Code,
    };

    Block::new(
        block_type,
        text_block.text(),
        BoundingBox::new(
            text_block.x0,
            text_block.y0,
            text_block.x1,
            text_block.y1,
        ),
    )
}
```

### Step 4: Update extractor.rs

Add pdfium preference:

```rust
#[cfg(feature = "pdfium")]
{
    if let Ok(backend) = PdfiumBackend::new(config.clone()) {
        info!("Using PdfiumBackend for high-quality extraction");
        return Self { backend: Box::new(backend), ... };
    }
    warn!("PdfiumBackend unavailable, falling back to lopdf");
}
```

### Step 5: Update Cargo.toml

```toml
[features]
default = ["pdfium", "lopdf"]  # pdfium preferred, lopdf as fallback
```

---

## Testing Strategy

1. **Unit tests**: Test PdfiumBackend with simple PDFs
2. **Integration tests**: Compare output with lopdf backend
3. **Quality evaluation**: Run eval_comprehensive.py
4. **Regression check**: Ensure 441 existing tests still pass

---

## Acceptance Criteria

- [ ] PdfiumBackend compiles with `--features pdfium`
- [ ] Falls back to lopdf when pdfium unavailable
- [ ] All 441 tests pass
- [ ] Quality >= 0.786 (no regression)
- [ ] API server uses pdfium when available
