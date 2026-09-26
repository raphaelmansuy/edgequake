# EdgeQuake on GCP — cheapest host (SPEC-148 Option A)

GCE VM **`elitizon-db`** (shared AGE + pgvector host) + Docker Compose (GHCR v0.27.0) in project `saas-app-001`.  
Caddy is the only public listener: **:80 always redirects to HTTPS**.

GCP objects (bucket, secrets, WIF, SAs, IP) use the **`edgequake-*`** prefix — not the spec number `eq148`. VPC is **`edgequake-host-vpc`** (leftover Option B already owns `edgequake-vpc`).

**Spec pack:** [specs/148-gcloud-hosting/README.md](../../specs/148-gcloud-hosting/README.md)

This module creates **`edgequake-*` plus VM `elitizon-db`**. VPC is **`edgequake-host-vpc`** (leftover Option B already owns `edgequake-vpc`). It does not import or destroy Cloud Run, `edgequake-db-vm`, leftover `edgequake-vpc`, or other apps in the shared project.

## Prerequisites

- `gcloud` authenticated to `raphael.mansuy@elitizon.com` / project `saas-app-001`
- Terraform >= 1.5
- Billing budgets **\$50 and \$100 USD** (existing alert is \$5 HKD)

```bash
gcloud config set project saas-app-001
gcloud services enable iap.googleapis.com --project=saas-app-001   # also enabled by Terraform
```

## Apply

```bash
cd deploy/gcp/terraform
cp terraform.tfvars.example terraform.tfvars   # already points at saas-app-001
terraform init
terraform plan
terraform apply
terraform output
```

## After apply

```bash
# IAP SSH (no public :22)
terraform output -raw ssh_iap_command

# HTTPS health (Let's Encrypt once hostname DNS points at external_ip):
IP=$(terraform output -raw external_ip)
curl -sI "http://demo.edgequake.com/health"   # 301/308 to https://demo.edgequake.com/...
curl -sf "https://demo.edgequake.com/health"

# Bootstrap admin password:
gcloud secrets versions access latest --secret=edgequake-bootstrap-admin-password --project=saas-app-001
```

Set GitHub Actions variables from outputs:

| GitHub variable | Terraform output |
|-----------------|------------------|
| `EDGEQUAKE_GCP_WIF_PROVIDER` | `workload_identity_provider` |
| `EDGEQUAKE_GCP_DEPLOY_SA` | `github_service_account` |
| `EDGEQUAKE_GCP_PROJECT` | `saas-app-001` |
| `EDGEQUAKE_GCP_ZONE` | `us-central1-a` |
| `EDGEQUAKE_GCP_INSTANCE` | `elitizon-db` |

Optional LLM keys (never commit):

```bash
printf '%s' "$OPENAI_API_KEY" | gcloud secrets versions add edgequake-openai-api-key --data-file=-
printf '%s' "$MISTRAL_API_KEY" | gcloud secrets versions add edgequake-mistral-api-key --data-file=-
# then: sudo /opt/edgequake/scripts/render-env.sh && sudo /opt/edgequake/scripts/deploy.sh
```

## Deploy a new pin

CI stays in **CI** and **Release — Docker (GHCR)** (quality gates, then GHCR). This workflow does not rebuild images and does not run `terraform apply`.

After a `vX.Y.Z` tag, **Release — Docker** must succeed. **Deploy GCP EdgeQuake** then installs that pin on `elitizon-db` (keyless WIF, IAP SSH, `install-release.sh`).

Manual rollback: GitHub → **Deploy GCP EdgeQuake** → `workflow_dispatch` with an already published `X.Y.Z`.

The workflow file must be on `edgequake-main`. WIF accepts only `refs/heads/edgequake-main`.

Or on the VM:

```bash
sudo /opt/edgequake/scripts/install-release.sh 0.27.0
```

`deploy.sh` is LD-15: `migrate dry-run` → `migrate` → `up` → HTTP 301 gate → `/health` → `\dx` (vector + age).

## Destroy this stack only

```bash
terraform destroy
# Data disk auto_delete=false; delete PD/snapshots only after a backup
```

Read the destroy plan. Terraform *state* prefix is still `eq148/` on `gs://saas-app-001-tf-state` (SPEC-148 isolation). Live GCP names are `edgequake-*` / `elitizon-db`.
