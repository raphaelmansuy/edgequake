---
title: Production hardening
description: The startup gates enforced when SSO is active, and a checklist for a safe production SSO deployment.
---

This page lists the startup checks that run when SSO is active, and the minimum checklist for production. The general checks are in the [security guide](../best-practices.md#startup-posture-checks).

## Startup gates

When an SSO provider is active, the server applies these gates at startup. Outside `EDGEQUAKE_DEV_MODE`, a failed gate stops the server. In dev mode, each failure is only a warning.

```mermaid
%%{init: {"theme":"base","themeVariables":{"primaryColor":"#E0E7FF","primaryBorderColor":"#6366F1","primaryTextColor":"#1E1B4B","secondaryColor":"#D1FAE5","secondaryBorderColor":"#10B981","secondaryTextColor":"#064E3B","tertiaryColor":"#FEF3C7","tertiaryBorderColor":"#F59E0B","tertiaryTextColor":"#6B7A90","lineColor":"#7A889C","clusterBkg":"rgba(99,102,241,0.07)","clusterBorder":"#A5B4FC","noteBkgColor":"#FEF9C3","noteTextColor":"#422006","textColor":"#6B7A90","titleColor":"#6B7A90","signalColor":"#7A889C","signalTextColor":"#6B7A90","loopTextColor":"#6B7A90","edgeLabelBackground":"#F1F5F9","actorLineColor":"#94A3B8"}}}%%
%% eq-theme:v1
flowchart TD
  A["SSO provider active at startup"] --> B{"Auth enabled?"}
  B -- "No, outside dev mode" --> X1["Refuse to start"]
  B -- "Yes, or dev mode" --> C{"Dev mode?"}
  C -- "No" --> D["Force strict tenant binding on"]
  C -- "Yes" --> W["Log warnings for any failed gate"]
  D --> E{"Every redirect URI https, localhost or 127.0.0.1?"}
  E -- "No, outside dev mode" --> X2["Refuse to start"]
  E -- "Yes" --> OK["Start"]
%% eq-classes
classDef eqLlm fill:#FEF3C7,stroke:#F59E0B,color:#451A03
classDef eqBad fill:#FEE2E2,stroke:#EF4444,color:#7F1D1D
class A eqLlm
class X1,W,X2 eqBad
```

Read it from the top. Outside dev mode, EdgeQuake turns strict tenant binding on for you, so you cannot turn it off. Redirect URIs must use `https`, except `http://localhost` and `http://127.0.0.1`, which are tolerated for local testing.

## Checklist

Treat these items as the minimum for a production SSO deployment.

- [ ] Keycloak 26.8.0 or later. This fixes organization enumeration (CVE-2026-4633). Pin the image digest.
- [ ] TLS everywhere. Set `sslRequired=external` in the realm. The dev overlay sets `none` through `EQ_KC_SSL_REQUIRED`, so never ship that setting.
- [ ] Replace the demo users and passwords, and `EQ_KC_CLIENT_SECRET`. Store the client secret in a secret manager (Helm: `api.extraSecretEnv` in `values-sso-keycloak.yaml.example`).
- [ ] Issuer URL equals Keycloak's frontend URL exactly. Run Keycloak with `KC_HOSTNAME` set.
- [ ] Allow `POST /api/v1/auth/oidc/backchannel-logout` from Keycloak through the ingress or a NetworkPolicy.
- [ ] Keep `EDGEQUAKE_OIDC_TRUST_EMAIL=false`, unless the IdP verifies emails. Keep `EDGEQUAKE_OIDC_LINK_POLICY=never`.
- [ ] Keep `EDGEQUAKE_OIDC_MAX_ROLE` at `admin` (the default), so the IdP cannot create owners. Raise it to `owner` only on purpose.
- [ ] Set `JWT_SECRET` to at least 32 bytes. The insecure default and shorter secrets are fatal outside dev mode.
- [ ] Set `EDGEQUAKE_CORS_ORIGINS` to an explicit list. With a non-local `DATABASE_URL`, startup refuses open CORS outside dev mode.
- [ ] Keep a break-glass local admin.
- [ ] Back up the Keycloak database. Realm import is create-only on an existing realm.
- [ ] For several replicas, note that SSO state, handoff codes, sessions and `jti` replay protection live in PostgreSQL (migrations 164 and 165). Any replica can complete a login that another replica started.
