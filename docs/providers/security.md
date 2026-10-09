---
title: Provider security
description: How EdgeQuake protects model API keys and provider URLs, and the settings that must be right before you expose the server.
---

Saved Connections hold API keys and URLs that point at other servers. This page lists the controls around them and the settings to check. The full threat model is in [Security](../security/index.md).

## Controls at a glance

| Control | What it does | Setting |
|---------|--------------|---------|
| Encrypted keys | Connection keys are stored with AES-256-GCM. API responses show only a fingerprint and `key_configured`. | `EDGEQUAKE_SECRETS_KEY` |
| Write-only keys | You can set or replace a key; you can never read it back. | none |
| URL checks | Saved and tested URLs go through an SSRF check. | `allow_private_network` per Connection |
| Admin only | Creating, testing and deleting Connections needs the admin role. | `EDGEQUAKE_AUTH_ENABLED` |
| Loopback publish | The quickstart publishes ports on `127.0.0.1` only. | `docker-compose.quickstart.yml` |

## Set the encryption key

`EDGEQUAKE_SECRETS_KEY` must hold 32 bytes: raw (32 characters), base64, or 64 hex characters. Without it, saving a key returns an error and no key is stored.

```bash
export EDGEQUAKE_SECRETS_KEY="$(openssl rand -base64 32)"
```

Keep this value. If it changes or is lost, stored keys cannot be decrypted and the affected roles fall back silently (see [Roles](roles.md)). `quickstart.sh` generates a new key on each run unless you export one first, so export your own before the first run. `EDGEQUAKE_SECRETS_KEY_ID` (default `v1`) labels the key; the current code decrypts with the one key in the environment, so it does not support rotation.

## Provider URL rules

| URL | Result |
|-----|--------|
| Not `http` or `https` | Rejected |
| `169.254.x.x` (cloud metadata), `fe80::/10`, `*.internal`, `kubernetes.default` | Always rejected |
| Loopback, `10.x`, `172.16-31.x`, `192.168.x`, `::1`, unique-local IPv6 | Rejected unless `allow_private_network` is true |
| Public host name or address | Allowed |

For Connections, `allow_private_network` defaults to true when `locality` is `local`. The Settings page always saves Connections as local with private network allowed. The check looks at the URL text and literal IP addresses; it does not resolve host names. See [SSRF defense](../security/best-practices.md#ssrf-defense-for-provider-urls).

## Server settings to check

- `EDGEQUAKE_AUTH_ENABLED=true` wins over `EDGEQUAKE_DEV_MODE=true`. The Docker quickstart sets dev mode on and auth off by default; turn auth on before you expose the port.
- `JWT_SECRET` must be at least 32 bytes and not the public default. The server refuses to start otherwise unless `EDGEQUAKE_DEV_MODE` is on.
- `EDGEQUAKE_SETUP_TOKEN`: when set, `POST /setup/initialize` requires the header `X-EdgeQuake-Setup-Token` with the same value, otherwise it answers 401.
- Production checklist: [Runtime auth hardening](../operations/runtime-auth-hardening.md), plus `EDGEQUAKE_CORS_ORIGINS`, `ALLOW_REGISTRATION=false` and `EDGEQUAKE_STRICT_STARTUP=1`.

Run `edgequake doctor` to check the key, the JWT secret, the database setting and the listen address in one step.
