# SPEC-008-05: Build Pipeline

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document specifies the build pipeline for the unified Astro + Starlight site. It covers local development, CI/CD automation, pre-build validation, and production build steps.

---

## 2. Build Flow

```
                         +--------------------+
                         |  Source Files       |
                         |  docs/ (symlink)    |
                         |  src/pages/         |
                         |  src/components/    |
                         +--------+-----------+
                                  |
                      +-----------v-----------+
                      |  Pre-build Validation  |
                      |  - Symlink check       |
                      |  - Frontmatter check   |
                      |  - Link check          |
                      +--------+--------------+
                               |
                    +----------v----------+
                    |  astro build         |
                    |  - Compile Astro/TSX |
                    |  - Process Starlight |
                    |  - Generate static   |
                    |  - Run Pagefind      |
                    +--------+------------+
                             |
                  +----------v----------+
                  |  Post-build Checks   |
                  |  - 404 page exists   |
                  |  - Sitemap valid     |
                  |  - CNAME present     |
                  +--------+------------+
                           |
                +----------v----------+
                |  dist/               |
                |  (Deploy artifact)   |
                +----------------------+
```

---

## 3. Pre-build Validation Script

```bash
#!/usr/bin/env bash
# scripts/prebuild-validate.sh
# Run before `astro build` to catch errors early

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
WEBSITE_DIR="$REPO_ROOT/edgequake-website"
DOCS_DIR="$REPO_ROOT/docs"
CONTENT_DOCS="$WEBSITE_DIR/src/content/docs"
errors=0

echo "=== Pre-build Validation ==="

# 1. Check symlink or content exists
if [ -L "$CONTENT_DOCS" ]; then
  resolved=$(cd "$WEBSITE_DIR/src/content" && readlink docs)
  if [ ! -d "$WEBSITE_DIR/src/content/$resolved" ]; then
    echo "FAIL: Symlink broken: $resolved"
    errors=$((errors + 1))
  else
    echo "OK: Symlink resolves to docs/"
  fi
elif [ -d "$CONTENT_DOCS" ]; then
  echo "OK: Content directory exists (copy mode)"
else
  echo "FAIL: No content at $CONTENT_DOCS"
  errors=$((errors + 1))
fi

# 2. Check frontmatter
missing_title=0
for file in "$DOCS_DIR"/**/*.md; do
  [ -f "$file" ] || continue
  basename=$(basename "$file")
  # Skip CHANGELOG and README
  if [ "$basename" = "CHANGELOG.md" ] || [ "$basename" = "README.md" ]; then
    continue
  fi
  if ! head -5 "$file" | grep -q '^title:'; then
    echo "WARN: Missing title frontmatter: $file"
    missing_title=$((missing_title + 1))
  fi
done
if [ $missing_title -gt 0 ]; then
  echo "WARN: $missing_title files missing title frontmatter"
fi

# 3. Check CNAME exists
if [ ! -f "$WEBSITE_DIR/public/CNAME" ]; then
  echo "FAIL: Missing public/CNAME"
  errors=$((errors + 1))
else
  echo "OK: CNAME present ($(cat "$WEBSITE_DIR/public/CNAME"))"
fi

echo ""
if [ $errors -gt 0 ]; then
  echo "FAIL: $errors critical errors found"
  exit 1
fi

echo "OK: All pre-build checks passed"
```

---

## 4. Build Command

```bash
# Full production build
cd edgequake-website
pnpm run build

# This executes:
# 1. Astro compiles all .astro pages (marketing + Starlight docs)
# 2. Starlight processes src/content/docs/ (the symlinked docs/)
# 3. Pagefind indexes all generated HTML pages
# 4. Output goes to dist/
```

### 4.1 Build Output Structure

```
dist/
+-- index.html              /
+-- demo/
|   +-- index.html          /demo/
+-- ecosystem/
|   +-- index.html          /ecosystem/
+-- enterprise/
|   +-- index.html          /enterprise/
+-- contact/
|   +-- index.html          /contact/
+-- 404.html                Custom 404
+-- docs/
|   +-- index.html          /docs/
|   +-- getting-started/
|   |   +-- installation/
|   |   |   +-- index.html
|   |   +-- quick-start/
|   |       +-- index.html
|   +-- concepts/
|   |   +-- ... (4 pages)
|   +-- ... (all 52 doc pages)
+-- pagefind/               Search index
|   +-- pagefind.js
|   +-- pagefind-ui.js
|   +-- pagefind-ui.css
|   +-- fragment/
|   +-- index/
+-- sitemap-index.xml       Sitemap
+-- favicon.svg
+-- CNAME
```

---

## 5. Local Development

```bash
# Start dev server with HMR
cd edgequake-website
pnpm dev

# Dev server behavior:
# - Marketing pages at http://localhost:4321/
# - Docs at http://localhost:4321/docs/
# - Hot reload on .astro, .tsx, .md file changes
# - Symlinked docs/ changes trigger reload instantly
```

### 5.1 Development Workflow

```
Developer edits docs/concepts/graph-rag.md
           |
           +---> Astro dev server detects change (via symlink)
           +---> Page /docs/concepts/graph-rag/ reloads in browser
           +---> < 1 second feedback loop
```

---

## 6. CI/CD Pipeline (GitHub Actions)

```yaml
# .github/workflows/deploy-website.yml
name: Deploy Website

on:
  push:
    branches: [main]
    paths:
      - "docs/**"
      - "edgequake-website/**"
  pull_request:
    paths:
      - "docs/**"
      - "edgequake-website/**"

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0 # Full history for lastUpdated dates

      - uses: pnpm/action-setup@v4
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: edgequake-website/pnpm-lock.yaml

      - name: Install dependencies
        working-directory: edgequake-website
        run: pnpm install --frozen-lockfile

      - name: Validate docs
        run: bash scripts/prebuild-validate.sh

      - name: Type check
        working-directory: edgequake-website
        run: pnpm run check

      - name: Build
        working-directory: edgequake-website
        run: pnpm run build

      - name: Verify build output
        run: |
          test -f edgequake-website/dist/index.html
          test -f edgequake-website/dist/docs/index.html
          test -f edgequake-website/dist/sitemap-index.xml
          test -d edgequake-website/dist/pagefind
          echo "OK: Build output verified"

  deploy:
    needs: build
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: pnpm/action-setup@v4
        with:
          version: 10

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: edgequake-website/pnpm-lock.yaml

      - name: Install and build
        working-directory: edgequake-website
        run: |
          pnpm install --frozen-lockfile
          pnpm run build

      # Deploy step depends on hosting provider
      # See 08-deployment-strategy.md for options
      - name: Deploy to Cloudflare Pages
        uses: cloudflare/wrangler-action@v3
        with:
          apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          accountId: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
          command: pages deploy edgequake-website/dist --project-name=edgequake
```

### 6.1 CI Triggers

| Trigger                                        | Action                               |
| ---------------------------------------------- | ------------------------------------ |
| Push to `main` changing `docs/**`              | Build + deploy (docs content change) |
| Push to `main` changing `edgequake-website/**` | Build + deploy (site code change)    |
| Pull request touching either path              | Build only (verify, no deploy)       |

### 6.2 Build Cache Strategy

```yaml
# Cache node_modules and Astro build cache
- uses: actions/cache@v4
  with:
    path: |
      edgequake-website/node_modules
      edgequake-website/.astro
    key: ${{ runner.os }}-astro-${{ hashFiles('edgequake-website/pnpm-lock.yaml') }}
    restore-keys: ${{ runner.os }}-astro-
```

---

## 7. Build Performance Targets

| Metric             | Target       | Rationale                                 |
| ------------------ | ------------ | ----------------------------------------- |
| Cold build time    | < 60 seconds | 7 marketing + 52 docs + Pagefind indexing |
| Incremental build  | < 15 seconds | Only changed pages rebuilt                |
| Dev server startup | < 3 seconds  | Fast developer feedback                   |
| Build output size  | < 20 MB      | Static HTML + search index                |

---

## 8. Makefile Integration

Add website build commands to the project root Makefile:

```makefile
# Website commands
website-dev:
	cd edgequake-website && pnpm dev

website-build:
	bash scripts/prebuild-validate.sh
	cd edgequake-website && pnpm run build

website-preview:
	cd edgequake-website && pnpm preview

website-check:
	cd edgequake-website && pnpm run check
```

---

## 9. Cross-References

- [00-overview.md](./00-overview.md) — Goal G8: single deployment point
- [02-single-source-strategy.md](./02-single-source-strategy.md) — Symlink and sync-docs.sh scripts
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Project structure and dependencies
- [08-deployment-strategy.md](./08-deployment-strategy.md) — Hosting provider and deploy target
- [09-migration-roadmap.md](./09-migration-roadmap.md) — When CI/CD is set up in migration phases
