# 06 — E2E matrix and runbook

> **Cross-refs:** [Laws](01-first-principles.md) · [Operator README](../../deploy/gcp/README.md)

## E2E matrix

| ID | Gate | Pass |
|----|------|------|
| E2E-148-01 | Terraform validate / plan | No Cloud SQL/Run/GKE resources; objects `edgequake-*`; VM `elitizon-db` |
| E2E-148-02 | IAP SSH | `gcloud compute ssh --tunnel-through-iap` succeeds; public `:22` from `0.0.0.0/0` **absent** on `edgequake-host-vpc` |
| E2E-148-03 | Extensions | `docker compose exec postgres psql … -c '\dx'` contains **vector** and **age** |
| E2E-148-04 | HTTP redirect | `curl -sI http://$IP/health` → **301 or 308** and `Location:` starts with `https://`. HTTP **200 is fail** |
| E2E-148-05 | HTTPS health | `curl -skf https://$IP/health` JSON `status` healthy (self-signed: `-k`) |
| E2E-148-06 | Auth | Unauthenticated UI/API is not open-admin (`DEV_MODE` false) |
| E2E-148-07 | Ports | Host has no public listener on 5432 / 8080 / 3000 |
| E2E-148-08 | Deploy script | `deploy.sh` dry-run then migrate then up; dry-run failure exits 1 before apply; second run idempotent |
| E2E-148-09 | Isolation | `gcloud run services list` still shows sibling services after apply |

## Operator runbook

### Apply

```bash
gcloud config set project saas-app-001
cd deploy/gcp/terraform
terraform init && terraform plan && terraform apply
terraform output
```

### SSH

```bash
terraform output -raw ssh_iap_command
# or
gcloud compute ssh elitizon-db --zone=us-central1-a --project=saas-app-001 --tunnel-through-iap
```

### Read secrets (once, locally)

```bash
gcloud secrets versions access latest --secret=edgequake-bootstrap-admin-password --project=saas-app-001
```

### Smoke

```bash
IP=$(terraform output -raw external_ip)
curl -sI "http://${IP}/health"          # must be 301/308
curl -skf "https://${IP}/health"        # must be healthy JSON
# on VM:
sudo docker compose -f /opt/edgequake/compose/docker-compose.yml exec postgres \
  psql -U edgequake -d edgequake -c '\dx'
```

### Deploy a new pin

GitHub → Actions → **Deploy GCP EdgeQuake** → `workflow_dispatch` with `version` (e.g. `0.26.5`).

Manual:

```bash
sudo EDGEQUAKE_VERSION=0.26.5 /opt/edgequake/scripts/deploy.sh
```

### Stop for cost (dev)

```bash
gcloud compute instances stop elitizon-db --zone=us-central1-a --project=saas-app-001
# disk + IP + snapshots still bill
```

Static IP **in use** bills at attached-VM rate; reserved unused IPs bill more — do not leave the address reserved with the VM deleted without releasing it.

## Edge cases

| EC | Case | Mitigation |
|----|------|------------|
| EC1 | Let’s Encrypt on a bare IP | Impossible; stay on `tls internal` until DNS A record |
| EC2 | HSTS too early | Do not send HSTS until hostname + public CA |
| EC3 | API starts before migrate | `deploy.sh` ordering; API exit 78 is a signal, not a silent fix |
| EC4 | OOM on e2-medium | Kill ingest; do not silently downsize to e2-small as default |
| EC5 | GHCR pull fail | Public images; retry; do not fall back to `latest` |
| EC6 | IAP API disabled | Terraform enables `iap.googleapis.com` |
| EC7 | Shared-project `terraform destroy` | State prefix `eq148/` only; still **read the plan** |
| EC8 | OpenAI secret UNSET | `render-env.sh` selects `mock` + `EDGEQUAKE_ALLOW_MOCK_PROVIDER=1` so v0.26.5 can boot; ingest is not a real LLM until a secret version is added and `deploy.sh` is re-run |
| EC9 | Missing `EDGEQUAKE_CORS_ORIGINS` | v0.26.5 exits: refuse open CORS on non-local `DATABASE_URL`. Overlay always sets it to the public origin |
| EC10 | `migrate dry-run` exits non-zero | `deploy.sh` logs `FAIL: migration dry-run failed`, exits 1, and does not run `migrate` apply or print `DEPLOY_OK` |
