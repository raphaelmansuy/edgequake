# LENS — Design (Admin + Dashboard UX)

> **Laws**: LAW-29, LAW-31  
> **Findings**: `iss087_admin_anon_filter`, `iss087_stats_n1`

---

## 1. Question

How should the UI communicate guest identity and stats degradation without adding dashboard clutter?

---

## 2. Users panel (#335)

### Problems today

- `anon_*` rows look like real users.  
- No badge, no filter, no empty-state explanation.  
- Operators infer breach or open registration.

### Design rules

1. **Default list**: real (login-capable) users only.  
2. **Optional toggle**: “Show guest / anonymous system accounts”.  
3. When shown: badge **Guest** (neutral, not error-red); subtitle `system · cannot sign in`.  
4. Do not use card spam, pill clusters, or scary “security alert” chrome for expected guest rows.  
5. If shared guest is the only anonymous row, consider hiding it entirely unless toggle on.

### Copy

| State | Copy |
|-------|------|
| Empty real users | “No users yet. Invite or register an account.” |
| Toggle on, guest visible | “Guest — used for unauthenticated chat when auth is off.” |
| Strict mode / anon disabled | No guest row; chat prompts sign-in |

---

## 3. Dashboard stats (#334)

### Problems today

- Cold cache + scale → 500 / blank KPIs.  
- Stale cache → possible silent wrong numbers with `stale` flag.

### Design rules

1. Keep existing **stale** treatment (P-G13) — one quiet banner, not a modal.  
2. After Wave 1, stale should be rare; do not invent a second loading UX.  
3. KPI labels: if SSOT becomes “chunks with embeddings ≈ chunk count”, ensure tooltip/help text matches (`embedding_count` meaning).  
4. Never show fake zeros while a request is still in flight — prefer skeleton / previous value.

---

## 4. Non-goals

- Redesigning the whole Settings IA  
- Marketing-style hero on admin pages  
- Purple glow “AI” badges for guest users

---

## 5. Acceptance for this lens

- [ ] Default Users view free of anon spam  
- [ ] Guest (if shown) clearly labeled  
- [ ] Stats error/stale states remain calm and rare after fix
