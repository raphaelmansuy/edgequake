# 004 - GitHub Actions Secrets and Workflow

## Workflow file

`/.github/workflows/publish-java-sdk.yml`

### Trigger

- Push tags matching `sdk-java-v*`
- Manual `workflow_dispatch`

### High-level CI steps

1. Checkout repository
2. Setup Java 17 with Maven cache
3. Inject OSSRH server credentials + GPG key via `actions/setup-java`
4. Validate package build
5. Deploy signed artifacts to OSSRH

## Why `actions/setup-java` is used for signing

It handles:

- temporary import of armored private key
- Maven `settings.xml` server credential mapping
- passphrase binding for signing phase

This reduces manual GPG management in workflow steps.

## Server ID contract

`pom.xml` distribution uses server ID `ossrh`.

Workflow must configure:

- `server-id: ossrh`
- `server-username: OSSRH_USERNAME`
- `server-password: OSSRH_TOKEN`

If these do not match, deploy will fail with auth/transport errors.

## Security model

- no credentials in repository files
- secrets only via GitHub encrypted secrets
- signatures generated in CI runtime only

