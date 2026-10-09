# 11 — Ops runbook

```bash
# Doctor
edgequake doctor
edgequake doctor --json

# Connections (auth on: admin JWT or master key)
curl -sS http://127.0.0.1:8080/api/v1/connections
curl -sS -X POST http://127.0.0.1:8080/api/v1/providers/test \
  -H 'Content-Type: application/json' \
  -d '{"shape":"openai_chat","base_url":"http://127.0.0.1:9050","model":"default"}'

# Secrets
export EDGEQUAKE_SECRETS_KEY=$(openssl rand -base64 32)
edgequake migrate   # 169
```

If doctor reports `secrets_key_missing`, keys cannot be written. Env providers still work.
