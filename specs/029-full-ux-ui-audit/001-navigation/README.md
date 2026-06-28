# 001 — Navigation Audit

**Scope:** Sidebar, header, breadcrumb, routing patterns, workspace selector  
**Files audited:**  
- [`src/components/layout/sidebar.tsx`](../../../../edgequake_webui/src/components/layout/sidebar.tsx)  
- [`src/components/layout/header.tsx`](../../../../edgequake_webui/src/components/layout/header.tsx)  
- [`src/components/layout/dynamic-breadcrumb.tsx`](../../../../edgequake_webui/src/components/layout/dynamic-breadcrumb.tsx)  
- [`src/app/(dashboard)/layout.tsx`](../../../../edgequake_webui/src/app/(dashboard)/layout.tsx)

---

## Contents

- [001-sidebar-nav-audit.md](001-sidebar-nav-audit.md) — Full sidebar analysis, grouping, ARIA

---

## TL;DR

The sidebar navigation carries 10 items in a flat list without grouping. The header + breadcrumb + active sidebar state creates **three competing location signals**. The workspace/tenant selector is buried in the header and duplicated in the sidebar. These are the most impactful navigation issues to fix.
