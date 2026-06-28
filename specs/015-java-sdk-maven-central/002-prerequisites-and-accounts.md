# 002 - Prerequisites and Accounts

## Repository prerequisites

- You have maintainer access to `raphaelmansuy/edgequake`
- You can create/update repository secrets
- You can push tags to trigger release workflows

## Sonatype / Maven Central prerequisites

1. Create/sign in to Sonatype Central account
2. Ensure namespace `io.edgequake` is owned/verified for your account
3. Generate a publishing token (username + token/password)

These values are used in CI as:

- `OSSRH_USERNAME`
- `OSSRH_TOKEN`

## GPG prerequisites

Maven Central requires artifact signatures. You need:

- one private key suitable for release signing
- passphrase for that key
- exported ASCII-armored private key for GitHub Secrets

## Local tooling prerequisites

- `gpg` installed
- `mvn` (Maven 3.8+)
- JDK 17+
- `gh` CLI (optional, for release/issue automation)

