# SPEC-008-02: Single Source of Truth Strategy

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Problem Statement

EdgeQuake documentation lives in `docs/` (52 markdown files). The Starlight integration in the unified Astro project expects content in `src/content/docs/`. Naively copying files creates duplication — two copies to maintain, inevitable drift.

**Requirement:** `docs/` is the single source of truth. The published site must read from it directly.

---

## 2. Strategy: Filesystem Symlinks

Starlight reads content from `edgequake-website/src/content/docs/`. We symlink this directory to the repository root `docs/` folder.

```
Repository Root
|
+-- docs/                             <-- Single source of truth
|   +-- getting-started/
|   +-- concepts/
|   +-- architecture/
|   +-- deep-dives/
|   +-- api-reference/
|   +-- tutorials/
|   +-- operations/
|   +-- security/
|   +-- integrations/
|   +-- comparisons/
|   +-- troubleshooting/
|   +-- fixes/
|   +-- cookbook.md
|   +-- faq.md
|   +-- features.md
|
+-- edgequake-website/                <-- Unified Astro + Starlight project
    +-- astro.config.mjs
    +-- package.json
    +-- src/
    |   +-- assets/                   <-- Logos, images
    |   +-- components/               <-- Astro + React island components
    |   +-- content/
    |   |   +-- docs/                 <-- SYMLINK -> ../../../docs/
    |   |       +-- (resolved from docs/)
    |   +-- layouts/                  <-- Page layouts
    |   +-- pages/                    <-- Marketing routes (/, /demo, etc.)
    |   +-- styles/                   <-- Global CSS
    |   +-- content.config.ts
    +-- public/
        +-- favicon.svg
        +-- CNAME
```

### 2.1 Symlink Creation

```bash
# One-time setup (run from repo root)
mkdir -p edgequake-website/src/content
ln -s ../../../docs edgequake-website/src/content/docs
```

This creates:

```
edgequake-website/src/content/docs -> ../../../docs
```

### 2.2 Why Symlinks Work

| Concern                               | Answer                                                                                                                            |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Does Astro follow symlinks?           | **Yes.** Node.js `fs` operations follow symlinks. Astro's content collection loader reads the resolved files.                     |
| Does Git track symlinks?              | **Yes.** Git stores symlinks as file-type symlink. `git clone` recreates them.                                                    |
| Does Vercel/CF Pages follow symlinks? | **Yes.** Build happens on the server where symlinks resolve to real files.                                                        |
| Cross-platform (Windows)?             | **Partial.** Git for Windows supports symlinks with `core.symlinks=true`. For Windows, use the build script fallback (Section 3). |

### 2.3 Git Configuration

Add to `.gitattributes`:

```
edgequake-website/src/content/docs symlink=true
```

---

## 3. Fallback: Build-Time Copy Script

For CI environments where symlinks are unreliable, a pre-build script copies files:

```bash
#!/usr/bin/env bash
# scripts/sync-docs.sh
# Syncs docs/ into edgequake-website/src/content/docs/ for CI builds

set -euo pipefail

SRC_DIR="$(git rev-parse --show-toplevel)/docs"
DEST_DIR="$(git rev-parse --show-toplevel)/edgequake-website/src/content/docs"

# Only copy if destination is not already a symlink
if [ -L "$DEST_DIR" ]; then
  echo "* Symlink exists, skipping copy"
  exit 0
fi

echo "-> Syncing docs/ to edgequake-website/src/content/docs/"
mkdir -p "$DEST_DIR"
rsync -av --delete \
  --exclude='CHANGELOG.md' \
  --exclude='README.md' \
  "$SRC_DIR/" "$DEST_DIR/"

echo "* Synced $(find "$DEST_DIR" -name '*.md' | wc -l | tr -d ' ') markdown files"
```

### 3.1 When to Use Each Approach

```
+---------------------+     +---------------------------+
|  Local Development  |     |  CI/CD Build              |
|                     |     |                           |
|  Symlink (default)  |     |  Symlink if supported     |
|  Instant changes    |     |  OR sync script fallback  |
|  Zero latency       |     |                           |
+---------------------+     +---------------------------+
```

| Scenario              | Approach                               |
| --------------------- | -------------------------------------- |
| macOS/Linux local dev | Symlink (default)                      |
| GitHub Actions CI     | Symlink (Linux runner, works natively) |
| Vercel build          | Symlink (Linux container)              |
| Windows local dev     | Sync script fallback                   |

---

## 4. Frontmatter Handling

Starlight requires YAML frontmatter with at least a `title` field. Existing `docs/` files may or may not have frontmatter.

### 4.1 Strategy: Add Frontmatter to `docs/` Source Files

Since `docs/` is the source of truth, we add Starlight-compatible frontmatter directly to the source files. This is a one-time migration:

**Before:**

```markdown
# Entity Extraction

EdgeQuake's entity extraction pipeline...
```

**After:**

```markdown
---
title: Entity Extraction
description: How EdgeQuake extracts entities from documents using LLM-powered analysis.
---

# Entity Extraction

EdgeQuake's entity extraction pipeline...
```

### 4.2 Automated Frontmatter Injection Script

For files missing frontmatter, a script infers title from the first `# heading`:

```bash
#!/usr/bin/env bash
# scripts/inject-frontmatter.sh
# Adds title frontmatter to docs that lack it

set -euo pipefail

for file in docs/**/*.md; do
  # Skip if already has frontmatter
  if head -1 "$file" | grep -q '^---'; then
    continue
  fi

  # Extract title from first H1
  title=$(grep -m1 '^# ' "$file" | sed 's/^# //')

  if [ -n "$title" ]; then
    # Prepend frontmatter
    tmpfile=$(mktemp)
    printf '%s\n%s\n%s\n' '---' "title: \"$title\"" '---' > "$tmpfile"
    cat "$file" >> "$tmpfile"
    mv "$tmpfile" "$file"
    echo "* Added frontmatter to $file"
  fi
done
```

### 4.3 Frontmatter Compatibility

The frontmatter we add is **plain YAML** — compatible with:

- Starlight (Astro content collections)
- GitHub markdown renderer (ignores frontmatter)
- Any other docs tool
- VSCode markdown preview

---

## 5. Asset Handling

### 5.1 Images and Diagrams

If `docs/` files reference images, they use relative paths:

```markdown
![Architecture](./assets/architecture.png)
```

With symlinks, the relative path resolves correctly because the content directory IS `docs/`.

### 5.2 Site-Specific Assets

Assets specific to the published site (logo, favicon, custom CSS) live in the Astro project:

```
edgequake-website/
+-- src/
|   +-- assets/
|       +-- logo-light.svg
|       +-- logo-dark.svg
+-- public/
    +-- favicon.svg
    +-- CNAME
```

---

## 6. Editing Workflow

```
Developer edits docs/concepts/graph-rag.md
          |
          +----> GitHub shows updated markdown
          |
          +----> Astro dev server (via symlink) auto-reloads
          |
          +----> CI rebuilds edgequake.com on push
```

**Single edit, three outputs.** No duplication, no sync step.

---

## 7. Validation

### 7.1 Pre-build Checks

The build pipeline validates:

1. **Symlink resolution**: All symlinked files exist
2. **Frontmatter presence**: Every `.md` file has a `title` field
3. **No broken links**: Internal links between docs resolve
4. **No orphan files**: Every markdown file appears in sidebar config

### 7.2 CI Check Script

```bash
#!/usr/bin/env bash
# scripts/validate-docs.sh

set -euo pipefail
errors=0

# Check symlink resolves
if [ -L edgequake-website/src/content/docs ]; then
  target=$(readlink edgequake-website/src/content/docs)
  if [ ! -d "edgequake-website/src/content/$target" ]; then
    echo "ERROR: Symlink broken: $target"
    errors=$((errors + 1))
  fi
fi

# Check all docs have frontmatter title
for file in docs/**/*.md; do
  if ! head -5 "$file" | grep -q '^title:'; then
    echo "ERROR: Missing title frontmatter: $file"
    errors=$((errors + 1))
  fi
done

if [ $errors -gt 0 ]; then
  echo "FAIL: $errors validation errors"
  exit 1
fi

echo "OK: All docs validated"
```

---

## 8. Cross-References

- [00-overview.md](./00-overview.md) — Goal G2: Zero content duplication
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Directory structure and content config
- [05-build-pipeline.md](./05-build-pipeline.md) — CI/CD integration of sync and validation scripts
- [07-content-authoring-standards.md](./07-content-authoring-standards.md) — Frontmatter schema details
- [10-content-mapping-matrix.md](./10-content-mapping-matrix.md) — Full file-to-section mapping
