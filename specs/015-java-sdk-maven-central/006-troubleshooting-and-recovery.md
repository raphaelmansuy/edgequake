# 006 - Troubleshooting and Recovery

## Error: 401 / 403 from OSSRH

Likely causes:

- wrong `OSSRH_USERNAME` or `OSSRH_TOKEN`
- namespace/group not authorized for account

Actions:

1. regenerate Sonatype publishing token
2. update GitHub secrets
3. retry with new tag

## Error: GPG signing failed

Likely causes:

- malformed armored private key in secret
- incorrect passphrase
- key import mismatch

Actions:

1. re-export private key with `--armor --export-secret-keys`
2. replace `OSSRH_GPG_SECRET_KEY`
3. verify passphrase in `OSSRH_GPG_SECRET_KEY_PASSWORD`

## Error: Javadoc/source/signature artifacts missing

Likely causes:

- plugins missing or misconfigured in `pom.xml`

Actions:

1. verify `maven-source-plugin` and `maven-javadoc-plugin` executions
2. verify `maven-gpg-plugin` runs in `verify`

## Workflow does not trigger

Likely causes:

- tag does not match `sdk-java-v*`

Actions:

1. create valid tag format
2. push tag

## Maven metadata / SCM rejection

Likely causes:

- stale repo URLs or invalid SCM block in `pom.xml`

Actions:

1. ensure repo points to `https://github.com/raphaelmansuy/edgequake`
2. ensure `scm` URL/connection/developerConnection are valid

## Recovery checklist after failed publish

1. inspect workflow logs
2. identify whether failure is auth, signing, metadata, or staging
3. apply minimal fix
4. rerun with a new version/tag
5. verify artifact availability before closing release task

