# Lens — Product Owner (SPEC-146)

## Outcome

Enterprise customers can put **mixed-classification documents in one workspace** and trust that viewers only see, retrieve, and cite what policy allows — without losing GraphRAG quality on the authorized set.

## Jobs to be done

1. Tag classification / project / share mode at upload (not after the fact).  
2. Manage workspace members, roles, and subject attributes in Settings.  
3. Publish policies from templates (advanced Cedar optional).  
4. Query and browse without learning that unauthorized docs exist.  
5. Quarantine unlabeled classified uploads and retry labeling.  
6. Break-glass for emergencies with loud audit — never silent.

## Personas

| Persona | Need |
|---------|------|
| Document manager | Labels at admit; quarantine clarity |
| Workspace admin | Members, roles, attrs, policies |
| Analyst (viewer) | Query without classification noise |
| Compliance / auditor | Audit trail; no title leaks |
| Platform engineer | Feature flag; migration honesty |

## Non-goals (v1)

- Encrypted vector search marketing claims.  
- Acc score improvements from security filters.  
- Replacing SSO/IdP (OIDC/oauth2-proxy remains edge).  
- Per-user private knowledge graphs as a product SKU.

## Success metrics (product)

| Metric | Gate |
|--------|------|
| Unauthorized context rate (harness) | TARGET 0 |
| Unauthorized title in UI/API | TARGET 0 |
| Legacy docs keep workspace share | Migration honesty |
| Upload labeling time-to-admit | Same order as today when defaults used |
| Acc impact | UNCONFIRMED — measure, don’t invent |

## Risks

| Risk | Mitigation |
|------|------------|
| Over-tightening breaks demos | `EDGEQUAKE_DOC_ABAC` off by default; legacy = workspace mode |
| Cedar too hard for admins | Template gallery; advanced opt-in |
| Support burden (404 confusion) | Operator docs: 404 means not found **or** unauthorized |
| False sense of security if only list filtered | Waves M2–M4 mandatory before “ABAC GA” |

## Rollout story

```ascii
  M0 flag + vocab unify
  M1 labels + list/detail ACL (visible security)
  M2–M3 retrieval/graph (real security)
  M4 MCP/cache/citations
  M5 break-glass + GA checklist
```

GA claim only after M4 DoD + harness TARGET 0.

## Cross-refs

- Why → [../00-why.md](../00-why.md)  
- Acceptance → [../10-acceptance.md](../10-acceptance.md)  
- UX → [004-ux.md](004-ux.md)  
