---
title: "Upgrade to EdgeQuake v0.40.11"
description: "Upgrade from v0.40.10 to v0.40.11. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.11

> **From:** v0.40.10 (schema **231**) · **To:** v0.40.11 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

v0.40.10 saved a Google Cloud project on a Vertex connection and let you edit that connection. A query on a Normal or image workspace still built `gemini-embedding-2` from the process environment, so the page reported that `GOOGLE_CLOUD_PROJECT` was not set. v0.40.11 uses the project saved on the connection for that query, for chat, and for image ingest. A stored service account is used when one is saved.

The organization model ceiling and the workspace model list are arranged by job. A model shows only the jobs it can do.

**SPEC-001 Acc** is not re-scored. The pack `medical-mid-20261024T031725Z` stays the current result: a statistical tie, not a win.
