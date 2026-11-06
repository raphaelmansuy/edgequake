---
title: "Upgrade to EdgeQuake v0.40.3"
description: "Upgrade from v0.40.2 to v0.40.3. Schema stays 227."
---

# Upgrade to EdgeQuake v0.40.3

> **From:** v0.40.2 (schema **227**) · **To:** v0.40.3 (schema **227**)

This patch adds no migration. `edgequake migrate` is a no-op on a database that is already at schema 227. Roll the images. Do not change the schema.

## Order of operations

```bash
# 1. Roll the API replicas to 0.40.3.
# 2. Roll the WebUI to 0.40.3.
```

You do not need a backup for a schema change. This cut does not write schema. Take a backup if your own policy requires one before an image roll.

## What changes for an operator

**Deployment › Connections can be saved while you are signed in.** v0.40.2 returned `422` "Providers are added on the deployment." for every create, update, delete and test, because the session's tenant was treated as a tenant-scoped write. The form now sends `?scope=deployment`. A platform admin stores a fleet row. A tenant header without that query still returns `connection_scope_global`. Someone who is not an admin and passes `scope=deployment` receives `403 deployment_admin_required`.

A Vertex service-account file saved this way is encrypted and is not returned in the response. After a process restart the row still reads "Service account saved", which means the same `EDGEQUAKE_SECRETS_KEY` decrypted it.

**Local development gets a stable secrets key.** `make dev`, `make dev-bg` and `make backend-bg` fill an empty `EDGEQUAKE_SECRETS_KEY` from `.edgequake-dev-secrets.env`. The file is created once, mode `600`, and never rewritten. An exported key wins. `make dev-auth` does not load it.

**demo.edgequake.com** must have the same variable or the Connections form returns `400` `EDGEQUAKE_SECRETS_KEY is required to store API keys`. The GCP render writes it into the API container. The value comes from Secret Manager secret `edgequake-secrets-key` when that secret has a real version. Otherwise it comes from `/opt/edgequake/secrets/edgequake-secrets-key` on the VM, which is created once, mode `600`, and never replaced. Replacing either copy makes saved credentials unreadable. See [the GCP host notes](../../deploy/gcp/README.md#provider-envelope-key).

**SPEC-001 Acc:** not re-scored on this cut. Mix retrieval is unchanged. The latest valid pack remains [`medical-mid-20261024T031725Z`](../../specs/001-benchmark/e2e/packs/history/medical-mid-20261024T031725Z/) (`valid: true`). EdgeQuake **0.8084**, LightRAG **0.7681**, delta **+0.0403**, 95% CI **[-0.0241, +0.1036]** (n_paired 72). The interval includes 0, so this is a statistical tie. EdgeQuake does not claim a win.
