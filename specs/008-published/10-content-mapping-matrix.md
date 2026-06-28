# SPEC-008-10: Content Mapping Matrix

| Field      | Value                        |
| ---------- | ---------------------------- |
| **Parent** | [SPEC-008](./00-overview.md) |
| **Status** | Draft                        |
| **Date**   | 2026-03-21                   |

---

## 1. Overview

This document provides a complete mapping of every source file to its published URL on the unified Astro + Starlight site. It covers both documentation pages (from `docs/`) and marketing pages (migrated from Next.js).

---

## 2. Marketing Pages (from Next.js)

| Source (Next.js)          | Source (Astro)               | Published URL  | Notes                               |
| ------------------------- | ---------------------------- | -------------- | ----------------------------------- |
| `app/page.tsx`            | `src/pages/index.astro`      | `/`            | Home page                           |
| `app/demo/page.tsx`       | `src/pages/demo.astro`       | `/demo/`       | React island: DemoModeSelector      |
| `app/ecosystem/page.tsx`  | `src/pages/ecosystem.astro`  | `/ecosystem/`  | Static Astro page                   |
| `app/enterprise/page.tsx` | `src/pages/enterprise.astro` | `/enterprise/` | Static Astro page                   |
| `app/contact/page.tsx`    | `src/pages/contact.astro`    | `/contact/`    | React island: ContactForm           |
| `app/docs/page.tsx`       | — (Starlight)                | `/docs/`       | Starlight handles this route        |
| `app/not-found.tsx`       | `src/pages/404.astro`        | `/404.html`    | Custom 404, `disable404Route: true` |

**Total marketing pages: 7** (6 Astro pages + 1 Starlight-handled docs index)

---

## 3. Documentation Pages (from `docs/`)

All files symlinked via `src/content/docs → ../../../docs`. Starlight generates URLs from the file path structure.

### 3.1 Getting Started (2 pages)

| Source File                            | Published URL                         | Sidebar Order |
| -------------------------------------- | ------------------------------------- | ------------- |
| `docs/getting-started/installation.md` | `/docs/getting-started/installation/` | 1             |
| `docs/getting-started/quick-start.md`  | `/docs/getting-started/quick-start/`  | 2             |

### 3.2 Concepts (4 pages)

| Source File                          | Published URL                       | Sidebar Order |
| ------------------------------------ | ----------------------------------- | ------------- |
| `docs/concepts/graph-rag.md`         | `/docs/concepts/graph-rag/`         | auto          |
| `docs/concepts/knowledge-graph.md`   | `/docs/concepts/knowledge-graph/`   | auto          |
| `docs/concepts/entity-extraction.md` | `/docs/concepts/entity-extraction/` | auto          |
| `docs/concepts/hybrid-retrieval.md`  | `/docs/concepts/hybrid-retrieval/`  | auto          |

### 3.3 Architecture (3 pages)

| Source File                             | Published URL                          | Sidebar Order |
| --------------------------------------- | -------------------------------------- | ------------- |
| `docs/architecture/overview.md`         | `/docs/architecture/overview/`         | auto          |
| `docs/architecture/data-flow.md`        | `/docs/architecture/data-flow/`        | auto          |
| `docs/architecture/lineage-tracking.md` | `/docs/architecture/lineage-tracking/` | auto          |

### 3.4 Deep Dives (13 pages)

| Source File                               | Published URL                            |
| ----------------------------------------- | ---------------------------------------- |
| `docs/deep-dives/chunking-strategies.md`  | `/docs/deep-dives/chunking-strategies/`  |
| `docs/deep-dives/community-detection.md`  | `/docs/deep-dives/community-detection/`  |
| `docs/deep-dives/cost-tracking.md`        | `/docs/deep-dives/cost-tracking/`        |
| `docs/deep-dives/embedding-models.md`     | `/docs/deep-dives/embedding-models/`     |
| `docs/deep-dives/entity-extraction.md`    | `/docs/deep-dives/entity-extraction/`    |
| `docs/deep-dives/entity-normalization.md` | `/docs/deep-dives/entity-normalization/` |
| `docs/deep-dives/gleaning.md`             | `/docs/deep-dives/gleaning/`             |
| `docs/deep-dives/graph-storage.md`        | `/docs/deep-dives/graph-storage/`        |
| `docs/deep-dives/lightrag-algorithm.md`   | `/docs/deep-dives/lightrag-algorithm/`   |
| `docs/deep-dives/pdf-processing.md`       | `/docs/deep-dives/pdf-processing/`       |
| `docs/deep-dives/pipeline-progress.md`    | `/docs/deep-dives/pipeline-progress/`    |
| `docs/deep-dives/query-modes.md`          | `/docs/deep-dives/query-modes/`          |
| `docs/deep-dives/vector-storage.md`       | `/docs/deep-dives/vector-storage/`       |

### 3.5 API Reference (4 pages)

| Source File                                             | Published URL                                          |
| ------------------------------------------------------- | ------------------------------------------------------ |
| `docs/api-reference/rest-api.md`                        | `/docs/api-reference/rest-api/`                        |
| `docs/api-reference/extended-api.md`                    | `/docs/api-reference/extended-api/`                    |
| `docs/api-reference/lineage-endpoints.md`               | `/docs/api-reference/lineage-endpoints/`               |
| `docs/api-reference/document-upload-quick-reference.md` | `/docs/api-reference/document-upload-quick-reference/` |

### 3.6 Tutorials (7 pages)

| Source File                                 | Published URL                              |
| ------------------------------------------- | ------------------------------------------ |
| `docs/tutorials/first-rag-app.md`           | `/docs/tutorials/first-rag-app/`           |
| `docs/tutorials/document-ingestion.md`      | `/docs/tutorials/document-ingestion/`      |
| `docs/tutorials/pdf-ingestion.md`           | `/docs/tutorials/pdf-ingestion/`           |
| `docs/tutorials/query-optimization.md`      | `/docs/tutorials/query-optimization/`      |
| `docs/tutorials/multi-tenant.md`            | `/docs/tutorials/multi-tenant/`            |
| `docs/tutorials/migration-from-lightrag.md` | `/docs/tutorials/migration-from-lightrag/` |
| `docs/tutorials/tracing-entity-sources.md`  | `/docs/tutorials/tracing-entity-sources/`  |

### 3.7 Operations (5 pages)

| Source File                             | Published URL                          |
| --------------------------------------- | -------------------------------------- |
| `docs/operations/configuration.md`      | `/docs/operations/configuration/`      |
| `docs/operations/deployment.md`         | `/docs/operations/deployment/`         |
| `docs/operations/monitoring.md`         | `/docs/operations/monitoring/`         |
| `docs/operations/performance-tuning.md` | `/docs/operations/performance-tuning/` |
| `docs/operations/metadata-debugging.md` | `/docs/operations/metadata-debugging/` |

### 3.8 Security (1 page)

| Source File                       | Published URL                    |
| --------------------------------- | -------------------------------- |
| `docs/security/best-practices.md` | `/docs/security/best-practices/` |

### 3.9 Integrations (3 pages)

| Source File                           | Published URL                        |
| ------------------------------------- | ------------------------------------ |
| `docs/integrations/langchain.md`      | `/docs/integrations/langchain/`      |
| `docs/integrations/open-webui.md`     | `/docs/integrations/open-webui/`     |
| `docs/integrations/custom-clients.md` | `/docs/integrations/custom-clients/` |

### 3.10 Comparisons (4 pages)

| Source File                                                      | Published URL                                                   |
| ---------------------------------------------------------------- | --------------------------------------------------------------- |
| `docs/comparisons/vs-traditional-rag.md`                         | `/docs/comparisons/vs-traditional-rag/`                         |
| `docs/comparisons/vs-graphrag.md`                                | `/docs/comparisons/vs-graphrag/`                                |
| `docs/comparisons/vs-lightrag-python.md`                         | `/docs/comparisons/vs-lightrag-python/`                         |
| `docs/comparisons/edgequake-vs-lightrag-superiority-analysis.md` | `/docs/comparisons/edgequake-vs-lightrag-superiority-analysis/` |

### 3.11 Troubleshooting (1 page)

| Source File                             | Published URL                          |
| --------------------------------------- | -------------------------------------- |
| `docs/troubleshooting/common-issues.md` | `/docs/troubleshooting/common-issues/` |

### 3.12 Fixes (1 page)

| Source File                                        | Published URL                                     |
| -------------------------------------------------- | ------------------------------------------------- |
| `docs/fixes/embedding-api-validation-error-fix.md` | `/docs/fixes/embedding-api-validation-error-fix/` |

### 3.13 Top-Level Reference Pages (4 pages)

| Source File                 | Published URL              | Sidebar Group |
| --------------------------- | -------------------------- | ------------- |
| `docs/cookbook.md`          | `/docs/cookbook/`          | Reference     |
| `docs/faq.md`               | `/docs/faq/`               | Reference     |
| `docs/features.md`          | `/docs/features/`          | Reference     |
| `docs/sqlx-offline-mode.md` | `/docs/sqlx-offline-mode/` | Reference     |

### 3.14 Excluded from Docs Site

| File                           | Reason                                     |
| ------------------------------ | ------------------------------------------ |
| `docs/linkedin-post-v0.4.0.md` | Marketing content, not documentation       |
| `docs/CHANGELOG.md`            | Changelog, not rendered as a docs page     |
| `docs/README.md`               | GitHub readme, not rendered as a docs page |

---

## 4. Summary

| Category            | Pages  |
| ------------------- | ------ |
| Marketing pages     | 7      |
| Getting Started     | 2      |
| Concepts            | 4      |
| Architecture        | 3      |
| Deep Dives          | 13     |
| API Reference       | 4      |
| Tutorials           | 7      |
| Operations          | 5      |
| Security            | 1      |
| Integrations        | 3      |
| Comparisons         | 4      |
| Troubleshooting     | 1      |
| Fixes               | 1      |
| Top-Level Reference | 4      |
| **Total**           | **59** |

---

## 5. URL Verification Checklist

Before DNS cutover (Phase 4), verify each URL responds with 200:

```bash
#!/usr/bin/env bash
# scripts/verify-urls.sh
# Run against preview deployment to verify all pages exist

BASE_URL="${1:-http://localhost:4321}"
errors=0

urls=(
  "/"
  "/demo/"
  "/ecosystem/"
  "/enterprise/"
  "/contact/"
  "/docs/"
  "/docs/getting-started/installation/"
  "/docs/getting-started/quick-start/"
  "/docs/concepts/graph-rag/"
  "/docs/concepts/knowledge-graph/"
  "/docs/concepts/entity-extraction/"
  "/docs/concepts/hybrid-retrieval/"
  "/docs/architecture/overview/"
  "/docs/architecture/data-flow/"
  "/docs/architecture/lineage-tracking/"
  "/docs/deep-dives/chunking-strategies/"
  "/docs/deep-dives/community-detection/"
  "/docs/deep-dives/cost-tracking/"
  "/docs/deep-dives/embedding-models/"
  "/docs/deep-dives/entity-extraction/"
  "/docs/deep-dives/entity-normalization/"
  "/docs/deep-dives/gleaning/"
  "/docs/deep-dives/graph-storage/"
  "/docs/deep-dives/lightrag-algorithm/"
  "/docs/deep-dives/pdf-processing/"
  "/docs/deep-dives/pipeline-progress/"
  "/docs/deep-dives/query-modes/"
  "/docs/deep-dives/vector-storage/"
  "/docs/api-reference/rest-api/"
  "/docs/api-reference/extended-api/"
  "/docs/api-reference/lineage-endpoints/"
  "/docs/api-reference/document-upload-quick-reference/"
  "/docs/tutorials/first-rag-app/"
  "/docs/tutorials/document-ingestion/"
  "/docs/tutorials/pdf-ingestion/"
  "/docs/tutorials/query-optimization/"
  "/docs/tutorials/multi-tenant/"
  "/docs/tutorials/migration-from-lightrag/"
  "/docs/tutorials/tracing-entity-sources/"
  "/docs/operations/configuration/"
  "/docs/operations/deployment/"
  "/docs/operations/monitoring/"
  "/docs/operations/performance-tuning/"
  "/docs/operations/metadata-debugging/"
  "/docs/security/best-practices/"
  "/docs/integrations/langchain/"
  "/docs/integrations/open-webui/"
  "/docs/integrations/custom-clients/"
  "/docs/comparisons/vs-traditional-rag/"
  "/docs/comparisons/vs-graphrag/"
  "/docs/comparisons/vs-lightrag-python/"
  "/docs/comparisons/edgequake-vs-lightrag-superiority-analysis/"
  "/docs/troubleshooting/common-issues/"
  "/docs/fixes/embedding-api-validation-error-fix/"
  "/docs/cookbook/"
  "/docs/faq/"
  "/docs/features/"
  "/docs/sqlx-offline-mode/"
)

echo "Verifying ${#urls[@]} URLs against $BASE_URL"
for url in "${urls[@]}"; do
  status=$(curl -o /dev/null -s -w "%{http_code}" "$BASE_URL$url")
  if [ "$status" != "200" ]; then
    echo "FAIL [$status]: $url"
    errors=$((errors + 1))
  fi
done

if [ $errors -gt 0 ]; then
  echo "FAIL: $errors URLs returned non-200"
  exit 1
fi
echo "OK: All ${#urls[@]} URLs return 200"
```

---

## 6. Cross-References

- [00-overview.md](./00-overview.md) — Total page count (59)
- [03-information-architecture.md](./03-information-architecture.md) — Sitemap and navigation structure
- [04-starlight-project-setup.md](./04-starlight-project-setup.md) — Sidebar configuration matching these categories
- [07-content-authoring-standards.md](./07-content-authoring-standards.md) — Frontmatter requirements for each file
- [09-migration-roadmap.md](./09-migration-roadmap.md) — Phase 4 URL verification before cutover
