# SPEC-008-07: Content Authoring Standards

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document defines authoring conventions for all markdown content in `docs/` and marketing copy in `edgequake-website/`. Consistent frontmatter, formatting, and linking ensure Starlight processes pages correctly and the site appears professional.

---

## 2. Frontmatter Schema

### 2.1 Required Fields (Docs)

Every file in `docs/` must have a YAML frontmatter block:

```yaml
---
title: Installation Guide # Required. Renders as page <h1>
description: >- # Required. Used for meta description and search
  Step-by-step guide to install EdgeQuake on Linux, macOS, and Docker.
---
```

### 2.2 Optional Fields (Docs)

```yaml
---
title: Installation Guide
description: Step-by-step installation guide.
sidebar:
  order: 1 # Controls sort order within sidebar group
  label: Install # Override sidebar display text (shorter)
  badge: New # Badge rendered next to sidebar label
tableOfContents:
  minHeadingLevel: 2 # Default is 2
  maxHeadingLevel: 3 # Default is 3
editUrl: false # Disable "Edit page" link for this page
lastUpdated: 2026-03-21 # Override auto-detected last-updated date
---
```

### 2.3 Starlight Frontmatter vs Custom Fields

Starlight validates frontmatter strictly. Only Starlight-recognized fields are allowed in docs pages. Custom fields should NOT be added unless the content collection schema is extended in `content.config.ts`.

---

## 3. File Naming Conventions

| Rule                                  | Example                                              |
| ------------------------------------- | ---------------------------------------------------- |
| Lowercase with hyphens                | `graph-rag.md`, not `GraphRAG.md`                    |
| No spaces or underscores              | `quick-start.md`, not `quick_start.md`               |
| No numeric prefixes for ordering      | Use `sidebar.order` frontmatter instead              |
| Category folders match sidebar groups | `docs/getting-started/`, `docs/concepts/`            |
| Index pages use `index.md`            | `docs/getting-started/index.md` for category landing |

---

## 4. Markdown Formatting

### 4.1 Headings

```markdown
# Never used in docs content (Starlight renders title as h1)

## Section Heading (h2 — appears in table of contents)

### Subsection Heading (h3 — appears in table of contents)

#### Detail Heading (h4 — does NOT appear in TOC by default)
```

Rule: Start content with `##` headings. Never use `# H1` in docs — it conflicts with the frontmatter `title`.

### 4.2 Code Blocks

Starlight uses Expressive Code for syntax highlighting.

````markdown
```rust title="src/main.rs" {3-5}
fn main() {
    let engine = EdgeQuake::new();
    // Highlighted lines 3-5
    engine.add_document("example.md");
    engine.process();
    println!("Done");
}
```
````

Supported features:

- `title="filename"` — Shows a filename tab above the block
- `{3-5}` — Line highlighting
- `ins={3}` / `del={5}` — Diff-style add/remove highlighting
- `// [!code focus]` — Focus annotation on specific lines

### 4.3 Links

**Between docs pages** — use relative paths:

```markdown
See the [installation guide](../getting-started/installation.md) for setup steps.
```

**To marketing pages** — use absolute paths:

```markdown
Visit the [demo page](/demo/) to try EdgeQuake.
```

**External links** — full URLs:

```markdown
Read the [Astro docs](https://docs.astro.build/) for more info.
```

### 4.4 Images

Store images alongside the markdown or in a shared assets directory:

```
docs/
  architecture/
    overview.md
    images/
      pipeline-diagram.png
```

Reference with relative paths:

```markdown
![Pipeline architecture](./images/pipeline-diagram.png)
```

### 4.5 Admonitions (Asides)

Starlight supports callout admonitions using the `:::` syntax:

```markdown
:::note
This is a note with helpful context.
:::

:::tip
A useful tip for better performance.
:::

:::caution
Be careful with this configuration.
:::

:::danger
This action is irreversible.
:::
```

### 4.6 Tables

Use standard markdown tables. Keep them readable in source:

```markdown
| Provider | Speed  | Cost |
| -------- | ------ | ---- |
| Ollama   | Fast   | Free |
| OpenAI   | Medium | $$$  |
```

---

## 5. Content Quality Checklist

Before merging a docs PR, verify:

- [ ] Frontmatter has `title` and `description`
- [ ] No `# H1` heading in body content
- [ ] Code blocks have language identifiers (`rust`, `bash`, `yaml`, etc.)
- [ ] Internal links use relative paths and resolve correctly
- [ ] Images have alt text
- [ ] No broken links (validated by CI script)
- [ ] Content reads clearly to a developer unfamiliar with the project
- [ ] Spelling and grammar are correct

---

## 6. Frontmatter Injection Script

For existing `docs/` files missing frontmatter, use the injection script from [02-single-source-strategy.md](./02-single-source-strategy.md):

```bash
# Inject title from first H1 heading for files missing frontmatter
bash scripts/inject-frontmatter.sh
```

The script derives `title` from the first `# Heading` and generates `description` from the first paragraph.

---

## 7. Writing Style Guide

| Principle | Guidance                                                                 |
| --------- | ------------------------------------------------------------------------ |
| Audience  | Developers and technical decision-makers                                 |
| Tone      | Professional, direct, concise                                            |
| Voice     | Active voice preferred                                                   |
| Tense     | Present tense for instructions ("Run the command")                       |
| Length    | Aim for scannable content — short paragraphs, bullet lists               |
| Jargon    | Define acronyms on first use (e.g., RAG: Retrieval-Augmented Generation) |

---

## 8. Cross-References

- [00-overview.md](./00-overview.md) — Goal G3: high-quality docs experience
- [02-single-source-strategy.md](./02-single-source-strategy.md) — Frontmatter injection script
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Content collection schema
- [10-content-mapping-matrix.md](./10-content-mapping-matrix.md) — All docs files and their published URLs
