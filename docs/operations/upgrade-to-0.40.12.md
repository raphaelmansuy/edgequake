---
title: "Upgrade to EdgeQuake v0.40.12"
description: "Upgrade from v0.40.11 to v0.40.12. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.12

> **From:** v0.40.11 (schema **231**) · **To:** v0.40.12 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

A projection delivery that failed the same way on every attempt used to be claimed forever. v0.40.12 quarantines it after 10 claimed attempts. Set `EDGEQUAKE_PROJECTION_MAX_ATTEMPTS=0` to keep the old unbounded retries. One failing delivery in a batch does not quarantine the others. The reason is on `projection_deliveries.receipt`. Recovery is in [Projection deliveries and the retry ceiling](../data-layer/postgres.md#projection-deliveries-and-the-retry-ceiling).

**SPEC-001 Acc** is not re-scored. Mix retrieval is unchanged. The pack `medical-mid-20261024T031725Z` stays the current result: a statistical tie, not a win.
