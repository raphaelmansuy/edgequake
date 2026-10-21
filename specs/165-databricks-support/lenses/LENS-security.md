# LENS — Security

Parent: [04 Gap matrix](../04-gap-matrix.md) · Laws: [01](../01-first-principles.md)

## Identities

| Identity | Use in v1 | Lifetime |
|----------|-----------|----------|
| App service principal (`DATABRICKS_CLIENT_ID` / `SECRET`) | Lakebase role and model bearer | Secret up to 730 days. Workspace token and database token about 60 minutes |
| Signed-in user (`x-forwarded-access-token`) | Not read in v1 | One request. Do not store it |
| Personal access token used for this probe | Not a product secret | Rotate it. It was pasted into chat |

The probe connected as the project owner (the user who created the project), with an OAuth token as the password. An App connects as its service principal. That role is created when the database is added to the App (`PGUSER` = client id). This study did not create an App, so that role was not created.

## Rules that keep secrets out of git

- `app.yaml` uses `valueFrom` for the Lakebase resource and the model resource.
- The launcher builds `DATABASE_URL` in memory. Logs use the existing `redact_database_url`.
- Do not commit the probe token, `PGPASSWORD`, or a credential `token` field.

## Privilege

The owner role is not a superuser and could `CREATE EXTENSION vector`. That is the right privilege for migrate. It cannot install AGE. That matches the allow-list.

## TLS

`sslmode=require` on every Lakebase connection. The probe used it.

## Shared data plane

v1 uses one database role for every EdgeQuake tenant. Unity Catalog row filters do not apply inside that database. Tenant isolation stays the EdgeQuake tenant id, as it does on Docker. LAW-165-7 keeps user-header impersonation out of v1 so a missing header cannot silently widen access.

## Gaps

G4, G5, G9, G14 in [04](../04-gap-matrix.md).
