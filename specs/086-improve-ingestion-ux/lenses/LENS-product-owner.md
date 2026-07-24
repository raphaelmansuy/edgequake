# LENS — Product Owner

**Job:** decide what “done” means for format-agnostic ingestion feedback and quality trust.  
**Cites:** LAW-22…28 · findings `ux086_*` · SPEC-048 PO outcomes

---

## 1. Jobs-to-be-done

| JTBD | User moment | Success look |
|------|-------------|--------------|
| **J1** Trust any format | Drops `.md` or `.pdf` | Same stage chrome within 2s of 202 |
| **J2** Spot real queue | Fairness delay | Honest “Queued run(s)” ≠ frozen fake Done |
| **J3** Avoid re-upload | MD looks stuck | Never re-uploads thinking it failed |
| **J4** Compare quality fairly | PDF 3k entities vs MD 35 | Understands density/content length, not “MD broken” |
| **J5** Leave and return | Refresh mid-run | ActiveRuns recovers from server state |

---

## 2. Outcomes (measurable)

| ID | Outcome | Gate |
|----|---------|------|
| PO-086-01 | Time-to-first-live-stage &lt; 2s after upload 202 (MD and PDF) | `ux086_e_md_live_stage` |
| PO-086-02 | No Done+Queued conflict in network/UI audit | `ux086_e_admit_404` |
| PO-086-03 | In-flight MD visible on Documents list before promote | `ux086_v_staging_list` |
| PO-086-04 | Poll-only path advances stages | `ux086_e_ws_gap` |
| PO-086-05 | Quality narrative uses density, not absolute entity equality | `ux086_v_density_gate` |

---

## 3. Anti-goals

- Shipping a prettier PDF panel while MD stays message-only  
- Fake percentages to hide fairness queues  
- Declaring quality parity by forcing MD entity count ≈ PDF  
- Forking cancel/fairness product rules (LAW-28)

---

## 4. Priority (RICE-lite)

| Item | Reach | Impact | Confidence | Effort | Rank |
|------|-------|--------|------------|--------|------|
| Staging list visibility | All MD/TXT uploads | High | High | S–M | **P0** |
| Store/poll merge | All non-PDF progress panels | High | High | S | **P0** |
| One presenter | All formats | High | High | M | **P0** |
| Stage-transition WS | Small MD docs | Med | High | S | P1 |
| Source_type markdown | Badges/filters | Med | Med | S | P2 |
| Density gate | Power users / QA | Med | Med | M | P2 |

---

## 5. Stakeholder narrative

> Ingestion wait is our most expensive UX. PDF accidentally got a luxury progress product; Markdown got a seeded “Queued…” string. SPEC-086 makes **format a detail under one progress contract**: same stepper, same visibility, same cancel rules — and quality judged by **density and structure**, not raw entity vanity metrics.
