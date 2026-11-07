---
title: "Upgrade to EdgeQuake v0.40.10"
description: "Upgrade from v0.40.9 to v0.40.10. Schema stays 231."
---

# Upgrade to EdgeQuake v0.40.10

> **From:** v0.40.9 (schema **231**) · **To:** v0.40.10 (schema **231**)

`edgequake migrate` is a no-op when the database is already at schema 231. Roll the API and WebUI images.

v0.40.9 is the release that adds **Edit** on Deployment › Providers. Its image publish was cancelled when the release quality-gates job reached 90 minutes during a cold compile. v0.40.10 is that same product, with the job limit raised to 180 minutes so the image can publish.

Open a provider, choose Edit, change the name, base URL, or key, and choose Update provider. For Vertex, the form also saves the Google Cloud project. Leave the key blank to keep the key the server already has.

**SPEC-001 Acc** is not re-scored. The pack `medical-mid-20261024T031725Z` stays the current result: a statistical tie, not a win.
