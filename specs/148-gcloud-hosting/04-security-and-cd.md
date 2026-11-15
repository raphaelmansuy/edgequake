# 04 — Security and CD

> **Cross-refs:** [Laws](01-first-principles.md) · [Runbook](06-e2e-and-runbook.md)

## Perimeter

| Surface | Control |
|---------|---------|
| SSH | IAP TCP forwarding; firewall `35.235.240.0/20` :22 ([docs](https://docs.cloud.google.com/iap/docs/using-tcp-forwarding)) |
| OS identity | Instance metadata `enable-oslogin=TRUE` ([docs](https://docs.cloud.google.com/compute/docs/oslogin/set-up-oslogin)) |
| HTTP | Public :80 **redirect only** (LAW-148-13) |
| HTTPS | Public :443; `tls internal` until `hostname` tfvar; then Caddy automatic HTTPS |
| HSTS | **Off** until a real hostname + Let’s Encrypt (HSTS + IP self-signed locks browsers) |
| Postgres | No public bind (LAW-148-8) |
| API/UI ports | Not published on the host; Caddy reverse-proxy only |
| VM disk | Shielded VM (secure boot, vTPM, integrity) |

## Secrets

| Secret | Use |
|--------|-----|
| `edgequake-postgres-password` | `POSTGRES_PASSWORD` / `DATABASE_URL` |
| `edgequake-jwt` | `JWT_SECRET` |
| `edgequake-bootstrap-admin-password` | `EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD` |
| `edgequake-master-api-key` | `EDGEQUAKE_MASTER_API_KEY` |
| `edgequake-openai-api-key` | Optional; skip in `.env` if unset/`UNSET` |
| `edgequake-mistral-api-key` | Optional `MISTRAL_API_KEY`; skip in `.env` if unset/`UNSET` |

VM SA: `roles/secretmanager.secretAccessor` **on these secrets**, plus `roles/storage.objectViewer` on the artifacts bucket, `roles/logging.logWriter`, `roles/monitoring.metricWriter`.

**Rotate leftover:** Cloud Run `edgequake-api` (v0.5.4) currently has a plaintext OpenAI key. Revoke that key at the provider. Do **not** copy it into `edgequake-*` secrets.

## Identity for humans

Grant operators (project or instance):

- `roles/iap.tunnelResourceAccessor`
- `roles/compute.osLogin` or `osAdminLogin`
- `roles/iam.serviceAccountUser` on `edgequake-gce@`

```bash
gcloud compute ssh elitizon-db --zone=us-central1-a --tunnel-through-iap --project=saas-app-001
```

## CD (secure updates)

```text
GitHub Actions (workflow_dispatch)
    google-github-actions/auth@v3  (WIF, id-token: write)
         │
         ▼
    gcloud compute ssh --tunnel-through-iap
         │
         ▼
    sudo /opt/edgequake/scripts/install-release.sh X.Y.Z
         snapshot-or-rely-on-schedule
         docker compose pull          # pinned tag
         edgequake migrate dry-run   # non-zero aborts before apply
         edgequake migrate            # LD-15
         compose up -d
         HTTP 301 gate
         curl -k https://127.0.0.1/health
         \dx  vector + age
```

**WIF:** pool `edgequake-github`, provider GitHub OIDC `https://token.actions.githubusercontent.com`, attribute condition `assertion.repository == "raphaelmansuy/edgequake" && assertion.ref == "refs/heads/edgequake-main"`. Bind `edgequake-github@` with `roles/iam.workloadIdentityUser`. **No JSON keys. No stored SSH keys.** IAP and OS Login for that SA are granted on VM `elitizon-db` only (IAP limited to port 22). Project SSH keys are blocked (`block-project-ssh-keys=TRUE`).

**GitHub SA IAM:** `roles/iap.tunnelResourceAccessor`, `roles/compute.osAdminLogin`, `roles/iam.serviceAccountUser` on the VM SA. **Not** `roles/owner`. **Not** `terraform apply` from this workflow on first landing.

## Rollback

1. Pin previous `EDGEQUAKE_VERSION` and re-run `deploy.sh`.
2. If schema/data is wrong: stop compose, restore `elitizon-db-data` from scheduled snapshot, migrate forward only if needed.

## What this is not

- Cloud Armor / HTTPS load balancer (cost; later)
- Binary authorization / GKE Binary Auth
- Customer-managed CMEK (default Google-managed encryption on PD/GCS)
