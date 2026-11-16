---
title: "Upgrade to EdgeQuake v0.40.9"
description: "Upgrade from v0.40.8 to v0.40.9. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.9

> **From:** v0.40.8 (schema **231**) · **To:** v0.40.9 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

**Deployment › Providers has Edit.** Open a provider, choose Edit, change the name, base URL, or key, and choose Update provider. For Vertex, the form also saves the Google Cloud project. Leave the key blank to keep the key the server already has. An environment provider keeps using that environment key until a new key is saved.

**Nothing else in the API.** Mix retrieval ranking is the same.

**SPEC-001 Acc** is not re-scored. The pack `medical-mid-20261024T031725Z` stays the current result: a statistical tie, not a win.
