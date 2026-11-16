---
title: "Upgrade to EdgeQuake v0.40.12"
description: "Upgrade from v0.40.11 to v0.40.12. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.12

> **From:** v0.40.11 (schema **231**) · **To:** v0.40.12 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

A hybrid query could die in under 5 seconds with `canceling statement due to statement timeout`. Typed vector search now uses the same 15 second statement budget as a graph read, applied as 14750ms, so Postgres still cancels a runaway statement. `EDGEQUAKE_VECTOR_QUERY_TIMEOUT_MS` overrides that budget.

Once `eq_graph_nodes` exists, a graph read whose cutover ledger is not `ready` fails closed. It does not scan Apache AGE label tables. Finish `edgequake migrate` before serving that database. A ready ledger keeps reading `eq_graph_*`.

**SPEC-001 Acc** is not re-scored. Mix ranking is unchanged. The pack `medical-mid-20261024T031725Z` stays the current result: a statistical tie, not a win.
