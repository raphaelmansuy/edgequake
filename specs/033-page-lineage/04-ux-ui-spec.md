# UX/UI Specification — SPEC-033 Page Lineage

## Role perspectives

This section is written from three lenses:

- **UX/UI Designer** — interaction design, visual hierarchy, accessibility
- **AI Engineer** — provenance surface, citation confidence, data shape
- **Full Stack Developer** — component layout, props, state, routing

---

## 1. Data Hierarchy Panel — Page-Grouped Mode

### 1.1 Current layout (flat, non-PDF)

```
┌─────────────────────────────────────────────────────────────┐
│ Data Hierarchy                                              ▲ │
├─────────────────────────────────────────────────────────────┤
│ ▼ 📄 m_renault_espace_rhn_fr_mai_2025  23 chunks · 542 ent │
│   ├── ▶ ◫ Chunk 0   L1-2 · 14 ent                          │
│   ├── ▶ ◫ Chunk 1   L3-8 · 6 ent                           │
│   ├── ▶ ◫ Chunk 2   L9-15 · 8 ent                          │
│   ...                                                       │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 New layout — page-grouped (PDF with page markers)

```
┌─────────────────────────────────────────────────────────────┐
│ Data Hierarchy                                              ▲ │
├─────────────────────────────────────────────────────────────┤
│ ▼ 📄 m_renault_espace_rhn_fr_mai_2025  23 chunks · 542 ent │
│                                                             │
│   ▼ 📑 Page 1   2 chunks · 14 ent          [→ p.1]        │
│   │   ├── ▶ ◫ Chunk 0   L1-2 · 14 ent   p.1  [→ p.1]     │
│   │   └── ▶ ◫ Chunk 1   L3-8 · 6 ent    p.1  [→ p.1]     │
│   │                                                         │
│   ▼ 📑 Page 2   1 chunk · 8 ent           [→ p.2]        │
│   │   └── ▶ ◫ Chunk 2   L9-15 · 8 ent   p.2  [→ p.2]     │
│   │                                                         │
│   ▼ 📑 Page 3   3 chunks · 21 ent         [→ p.3]        │
│       ├── ▶ ◫ Chunk 3   L16-22 · 7 ent  p.3  [→ p.3]     │
│       ├── ▶ ◫ Chunk 4   L23-30 · 8 ent  p.3  [→ p.3]     │
│       └── ▶ ◫ Chunk 5   L31-38 · 6 ent  p.3  [→ p.3]     │
│                                                             │
│   ... (20 more pages)                                       │
└─────────────────────────────────────────────────────────────┘
```

`[→ p.N]` = a compact deeplink badge (not a button, a `<Link>` tag).

### 1.3 Page Group Header — Component Spec

```
┌── PageGroupNode ───────────────────────────────────────────┐
│                                                             │
│  [icon: FileStack]  Page {N}     {M} chunks · {E} entities │
│                                              [→ p.N badge] │
└─────────────────────────────────────────────────────────────┘
```

| Property        | Value                                                          |
| --------------- | -------------------------------------------------------------- |
| Icon            | `Layers` (lucide-react)                                        |
| Label           | `Page {N}` — e.g. "Page 3"                                     |
| Meta badge      | `{M} chunks · {E} ent` — right-aligned muted text              |
| Deeplink badge  | `p.{N}` — compact, blue, absolute-right                        |
| Click action    | Navigate PDF to page N; update URL `?page=N`; DO NOT set chunk |
| Expand/collapse | Default collapsed for pages > 3; expanded for pages 1–3        |
| Keyboard        | `Enter` / `Space` toggles expand; `→` opens deeplink           |
| `aria-label`    | `"Page {N}: {M} chunks"`                                       |

### 1.4 Chunk Node — Changes to existing `ChunkTreeNode`

Add to the right side of the chunk row when `page_start` is present:

```
◫ Chunk {idx}   L{s}-{e} · {n} ent   [p.{N}]   [selected ●]
```

- `[p.{N}]` is a compact `<Link>` badge (blue, `text-xs`, `rounded`)
  navigating to `?chunk=<id>&page=N`.
- The badge replaces the current line-range display only when page data
  is available; otherwise line range is shown as before.

### 1.5 Interaction Design — Chunk Click with Page

When a user **clicks** a chunk node (not the page badge):
1. The chunk gets selected (highlighted in hierarchy tree and in markdown panel).
2. If `page_start` is present, the PDF viewer navigates to that page.
3. URL becomes `?chunk=<id>&page=N`.

When a user **clicks** the `[p.N]` badge on a chunk or page header:
1. The PDF viewer navigates to page N.
2. URL becomes `?page=N` (chunk is NOT selected/deselected).
3. The markdown panel does NOT highlight any chunk.

Rationale: The badge is a "jump to page" shortcut; the full row click is
"select chunk for highlighting".

---

## 2. PDF Viewer — Controlled Navigation

### 2.1 New `currentPage` Prop Visual Contract

```
Before SPEC-033:
  parent:   <PDFViewer file=... initialPage={3} />
            ↑ PDF opens on page 3; never updates again

After SPEC-033:
  parent:   <PDFViewer file=... initialPage={1} currentPage={activePageFromUrl} />
            ↑ PDF navigates whenever activePageFromUrl changes
```

### 2.2 Toolbar — No Visual Change

The toolbar prev/next buttons continue to update internal `pageNumber`.
If the user navigates the PDF manually, the URL is NOT updated (local
internal state only — no history pollution).

### 2.3 Component States

```
┌─────────────────────────────────────────────────────────┐
│  PDF Viewer                           [−][+][⤢][↓]      │
├─────────────────────────────────────────────────────────┤
│                                                         │
│    [◀]  Page 3 / 23  [▶]                                │
│                                                         │
│  ┌───────────────────────────────────────┐              │
│  │                                       │              │
│  │   PDF page 3 content renders here     │              │
│  │                                       │              │
│  └───────────────────────────────────────┘              │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

No structural change to the toolbar.  The only visible change is that
clicking a chunk in the hierarchy tree jumps the PDF to the chunk's page.

---

## 3. Side-by-Side Layout — Page Context

```
┌──────────────────────────────┬──────────────────────────────┐
│  PDF Document                │  Extracted Markdown           │
│  ─────────────────────────── │  ─────────────────────────── │
│                              │                               │
│  [◀]  Page 3 / 23  [▶]      │  # plaisir de conduite       │
│                              │  ## motorisation full hybrid │
│  ┌────────────────────────┐  │  ...                         │
│  │                        │  │  ████████████████████        │
│  │  PDF rendered page 3   │  │  ← highlighted chunk         │
│  │                        │  │                               │
│  └────────────────────────┘  │                               │
│                              │                               │
│  (PDF navigated to p.3 when  │  (Chunk C3 highlighted in    │
│   chunk C3 clicked)          │   markdown panel)             │
│                              │                               │
├──────────────────────────────┼──────────────────────────────┤
│                  Metadata Sidebar (collapsible)               │
│                  ▼ Data Hierarchy                             │
│                    ▼ Page 3  3 chunks · 21 ent    [→ p.3]    │
│                      ├── Chunk 3  L16-22 · 7 ent  p.3 [●]   │
│                      ├── Chunk 4  L23-30 · 8 ent  p.3       │
│                      └── Chunk 5  L31-38 · 6 ent  p.3       │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. Query Results — Page-Grouped Citations

### 4.1 DocumentsTab — with page grouping

```
┌──────────────────────────────────────────────────────────┐
│ ① m_renault_espace          25%  20×  ↗                  │
├──────────────────────────────────────────────────────────┤
│                                                           │
│  ┌─ Page 1 ─────────────────────────────────────────┐   │
│  │  §1  # motorisation ## full hybrid E-Tech 200 ch  │   │
│  │      ... énergie full hybrid + essence sans...    │   │
│  │                                     100%  p.1 ↗  │   │
│  └───────────────────────────────────────────────────┘   │
│                                                           │
│  ┌─ Page 3 ─────────────────────────────────────────┐   │
│  │  §2  # voyagez dans l'Espace **Nouveau Renault**  │   │
│  │      ...réinvente le SUV de 5 ou 7 places...      │   │
│  │                                      92%  p.3 ↗  │   │
│  │  §3  # plaisir de conduite ## motorisation...     │   │
│  │      ...full hybrid sans recharge | jusqu'à 80%.. │   │
│  │                                      81%  p.3 ↗  │   │
│  └───────────────────────────────────────────────────┘   │
│                                                           │
│  [Show 17 more passages ▼]                                │
└──────────────────────────────────────────────────────────┘
```

### 4.2 Page Group Sub-Header — Component Spec

```
┌─ Page N ─────────────────────────────────────────────────┐
│  label: "Page {N}"                                        │
│  icon:  BookOpen (lucide-react, 12px)                     │
│  style: text-xs font-semibold text-muted-foreground       │
│         uppercase tracking-wide bg-muted/20 rounded px-2  │
└───────────────────────────────────────────────────────────┘
```

### 4.3 Passage Row — `p.N` Badge as Deeplink

```
┌─────────────────────────────────────────────────────────┐
│ §N  [passage content truncated to ~100 chars...]         │
│                            [score%]  [p.N ↗]            │
└─────────────────────────────────────────────────────────┘
```

`[p.N ↗]` is a `<Link>` component:
- `href`: `/documents/{document_id}?chunk={chunk_id}&page={page_start}`
- style: `text-xs font-medium text-primary hover:underline flex items-center gap-0.5`
- icon: `ExternalLink` (12px) from lucide-react
- `title`: `"Open PDF at page {page_start}"`
- `aria-label`: `"Open document at page {page_start}"`

### 4.4 Non-PDF / No Page Fallback

When `page_start` is undefined for all chunks in a document, the
`DocumentsTab` renders in flat mode (current behaviour):

```
┌──────────────────────────────────────────────────────────┐
│ ① some_markdown_doc          85%  5×  ↗                  │
├──────────────────────────────────────────────────────────┤
│  §1  content of chunk 1...                    85%        │
│  §2  content of chunk 2...                    79%        │
└──────────────────────────────────────────────────────────┘
```

---

## 5. Navigation Flow Diagrams

### 5.1 Hierarchy → PDF

```
[User sees tree]                    [User sees PDF]
      │                                    │
      │  clicks "Chunk 3  p.3 [→]"        │
      │──────────────────────────────────►│
      │                                    │
      │  URL: ?chunk=C3&page=3            │
      │  PDF viewer: page 3               │
      │  Markdown: chunk C3 highlighted   │
      │                                   │
```

### 5.2 Citation → PDF

```
[Query result]                      [Document view]
      │                                    │
      │  clicks "p.3 ↗" badge             │
      │──────────────────────────────────►│
      │                                    │
      │  URL: /documents/D?chunk=C&page=3 │
      │  PDF viewer: page 3               │
      │  Markdown: chunk C highlighted    │
      │                                   │
```

### 5.3 Share / Bookmark

```
User A shares:  /documents/D?chunk=C&page=3
                                │
                                ▼
                          User B opens URL
                          PDF viewer: page 3  (via pageFromUrl)
                          Chunk C selected    (via chunkIdFromUrl)
                          Markdown highlighted
```

---

## 6. Accessibility Checklist

| Element                | ARIA role         | Keyboard        | Focus visible |
| ---------------------- | ----------------- | --------------- | ------------- |
| Page group header      | `button` (expand) | `Enter`/`Space` | ✅ ring-2      |
| Page deeplink badge    | `link`            | `Enter`         | ✅ ring-2      |
| Chunk node row         | `button`          | `Enter`/`Space` | ✅ ring-2      |
| Chunk `p.N` badge      | `link`            | `Enter`         | ✅ ring-2      |
| Citation passage row   | `button`          | `Enter`/`Space` | ✅ ring-2      |
| Citation `p.N ↗` badge | `link`            | `Enter`         | ✅ ring-2      |

---

## 7. Responsive Behaviour

### Desktop (≥ 1024px)
Full side-by-side layout as specified above.  PDF viewer and markdown
panel visible simultaneously.

### Mobile / Tablet (< 1024px)
The current mobile layout shows a single tab view.  Page badges and page
group headers MUST still render.  The deeplink navigates to the document
detail page which will show the PDF tab (if implemented) or the markdown
tab.

### Metadata Sidebar Collapsed
When the metadata sidebar is collapsed, Data Hierarchy is hidden.
No functional impact — the page deeplinks in query citations still work.
