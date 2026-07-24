# SPEC-085 — First Principles

> **Status**: Active  
> **Product pin**: EdgeQuake v0.21.1  
> **Cross-refs**: [README](README.md) · [Register](01-alert-register.md) · [Roadmap](03-implementation-roadmap.md) · [Version pins](06-version-pins.md)  
> **Inherits**: [SPEC-083 laws](../083-improvements/00-first-principles.md) · [SPEC-084 LAW-9…14](../084-reliability-fix/00-first-principles.md)

---

## 1. WHY this pack exists

Dependabot shows **133 open alerts** across **29 packages**. Treating each alert as an isolated hotfix risks:

- Bumping the same package differently in webui vs website vs mcp  
- Hand-editing lockfiles and creating unreproducible trees  
- Jumping to canary majors (Next 16.3) when an LTS patch closes all GHSAs  
- Leaving `jsonwebtoken` **9.3** (API/workspace) beside **10.3+** (auth) — dual crypto pins  
- Closing Astro XSS alerts on 6.x when the remaining floor requires **≥7.1.0**

This pack collapses alerts into **one study per package**, a **security floor**, a **surface-scoped wave**, and a **regression gate**.

---

## 2. Laws (SPEC-083 + SPEC-084 + SPEC-085)

Reuse LAW-1…LAW-14. SPEC-085 adds:

```
  LAW-15  Security floor = max(first_patched) over all open GHSAs for the package
  LAW-16  One package → one pin SSOT across manifests (workspace / Maven property / pnpm override)
  LAW-17  Blast radius by surface (webui | website | mcp | ts-sdk | java/kotlin | rust-core | rust-sdk)
  LAW-18  Prefer patched minor; major only when floor requires it (Astro 7) with isolated wave
  LAW-19  Transitives: overrides + lock regenerate; never hand-edit lockfiles
  LAW-20  Alert closed only after version proof + surface regression gate
  LAW-21  No dual auth crypto pins (jsonwebtoken 9.x + 10.x forbidden)
```

### ASCII: laws → surfaces

```
                 +------------------+
                 | LAW-15 Floor     |
                 +--------+---------+
                          |
     +--------------------+--------------------+
     |                    |                    |
     v                    v                    v
 +--------+         +-----------+         +-----------+
 | LAW-16 |         | LAW-17/18 |         | LAW-21    |
 | Pin    |         | Surface   |         | JWT SSOT  |
 | SSOT   |         | + majors  |         |           |
 +---+----+         +-----+-----+         +-----+-----+
     |                    |                    |
     +----------+---------+----------+---------+
                |
                v
         +-------------+
         | LAW-19/20   |
         | Lock+Gate   |
         +-------------+
```

---

## 3. SOLID mapping (how we implement)

| Letter | Meaning here | Shared primitives (DRY) |
|--------|--------------|-------------------------|
| **S** | One package study owns upgrade + residual risk | `packages/PKG-*.md` |
| **O** | Surfaces extend via playbooks, not copy-paste recipes | [05-surface-playbooks.md](05-surface-playbooks.md) |
| **L** | Java and Kotlin honor the same Jackson floor | `${jackson.version}` |
| **I** | Narrow verification per surface | Wave gates in [04-verification-matrix.md](04-verification-matrix.md) |
| **D** | Dependabot/GHSA + resolved lock versions are dependency SSOT | Register floors, not tribal knowledge |

Anti-patterns banned:

- One Dependabot PR per alert when one pin closes N GHSAs  
- Editing `pnpm-lock.yaml` / `Cargo.lock` by hand  
- “Latest major” when a patched minor satisfies LAW-15  
- Leaving `bun.lock` as authority beside `pnpm-lock.yaml`  
- Dismissing alerts without version proof (LAW-20)

---

## 4. Locked architectural decisions

1. **Next**: pin **`16.2.11+`** (Active LTS security release). Reject canary as the security floor.  
2. **Astro**: upgrade to **≥7.1.0** (latest stable preferred, e.g. 7.1.3). Document [v7 migration](https://docs.astro.build/en/guides/upgrade-to/v7/).  
3. **Jackson**: single property bump to **≥2.18.9** in both SDKs.  
4. **jsonwebtoken**: unify to **≥10.3** with `aws_lc_rs`; delete 9.3 workspace/API pins.  
5. **Transitives**: extend webui `pnpm.overrides` pattern; npm `overrides` for mcp/ts-sdk.  
6. **This workstream**: documentation under `specs/085-fix-security/` first; code follows the roadmap.

---

## 5. Exploitability classes (for prioritization)

| Class | Meaning | Default wave bias |
|-------|---------|-------------------|
| **R-prod** | Reachable in production request path | Wave 0–1 |
| **R-ssr** | Server-side render / image / rewrite | Wave 0–2 |
| **D-dev** | Dev server / test UI only (local attacker) | Wave 0 if Critical, else Wave 6 |
| **T-build** | Build-time / CI transitive | Wave 6 |
| **S-sdk** | Client SDK dependency (consumer apps) | Wave 3–4 |

---

## 6. Verification pin

| Field | Value |
|-------|-------|
| Tag | v0.21.1 |
| Audit date | 2026-07-24 |
| Open alerts at audit | 133 |
| Unique packages | 29 |
| Code fixes in this pass | **None** (docs + plan only) |
