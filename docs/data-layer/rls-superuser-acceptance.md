---
title: "RLS superuser acceptance"
description: "Decision record GAP-091-12: why EdgeQuake first treated Row-Level Security as defense in depth, and how migration 167 later made RLS enforce on scoped data-access transactions."
---

# RLS superuser acceptance (GAP-091-12)

**Status:** Accepted on 2026-07-30 (SPEC-091 IW0). **Partly superseded** by migration 167. Read [What changed with migration 167](#what-changed-with-migration-167) first.

**Decision owner:** EdgeQuake maintainers.

Row-Level Security (RLS) is a PostgreSQL feature that filters the rows each query can see, based on policies on the table. This record explains how EdgeQuake relies on RLS and where the application layer still has to enforce isolation.

## Context at the time of the decision

- Tenant and workspace policies existed (migrations 081 and 096). A test suite (`e2e_postgres_rls`) ran them in CI against a non-superuser role named `app_user`.
- In production, the server connected as the database owner (`edgequake`). PostgreSQL superusers and table owners skip RLS unless the table uses `FORCE ROW LEVEL SECURITY`. So RLS did nothing on the production connection.
- Apache AGE stores the whole knowledge graph in one global graph. You cannot express per-tenant label policies there. That stayed out of scope (GAP-091-15).

## Decision

RLS was accepted as defense in depth. The application layer was named the enforcement boundary:

1. Fail-closed scope headers. A malformed `X-Tenant-ID` or `X-Workspace-ID` never matches everything (`middleware::ScopeHeader`, `isolation_context`, `task_scope`, `query_request_builder`).
2. Task scope checks always run (`task_scope::get_task_for_context`). A request without headers resolves to the built-in default scope on purpose.
3. Relational isolation is covered by `e2e_tenant_isolation` and `contract_spec091_strict_scope_headers`.
4. The RLS suite stayed in CI, so a future non-superuser runtime role could turn it on without policy changes.

## What changed with migration 167

Migration 167 created the role `edgequake_tenant_access` with `NOLOGIN NOSUPERUSER NOBYPASSRLS NOINHERIT`. It also added, per protected table, a restrictive policy (`tenant_access_guard`) that requires the current tenant and, if set, the current workspace, plus a permissive policy (`tenant_access_allow`) for that role. Both tenant-scoped and workspace-only tables are covered. Full policy text is in `edgequake/migrations/167*.sql` and summarized in [postgres.md](./postgres.md#tenant-isolation).

The Rust code in `rls.rs` now runs scoped data-access transactions as that role: it does `SET LOCAL ROLE edgequake_tenant_access`, calls `set_tenant_context`, and fails if the connected role is a superuser or has `BYPASSRLS`. Because the role cannot bypass RLS, the policies now enforce on those transactions, even when the login role is the owner.

What still holds from the original decision:

- Administration and queue connections keep their privileged path.
- Not every query goes through a scoped transaction, so every scoped read path must still filter in the application.
- The AGE graph has no tenant policies by default. Isolation there is by `tenant_id` and `workspace_id` properties and application filters. An optional AGE RLS mode exists (`EDGEQUAKE_AGE_RLS=true`, AGE 1.7.0 or later). See [age.md](./age.md#tenant-isolation-in-the-graph).

## Consequences

- Do not rely on RLS as the only isolation mechanism for a new table. Add the table to the policy list in a migration and also filter in code.
- Production must not connect as a superuser or a role with `BYPASSRLS`. The code rejects that for scoped transactions.

## References

- `specs/091-simplify-data-layer/18-full-completeness-assessment.md` (GAP-091-12)
- `edgequake/crates/edgequake-api/tests/e2e_postgres_rls.rs` and `e2e_tenant_rls.rs` (RLS suites)
- `edgequake/crates/edgequake-api/tests/e2e_tenant_isolation.rs` (application-layer isolation suite)
- [llm-cache-scope.md](./llm-cache-scope.md), an adjacent scope decision (GAP-091-14)
