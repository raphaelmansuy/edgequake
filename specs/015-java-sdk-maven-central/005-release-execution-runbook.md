# 005 - Release Execution Runbook

## 1) Pre-flight checks (local)

From repo root:

```bash
cd sdks/java
mvn -Dmaven.test.skip=true clean package
```

This validates package assembly before CI publish.

## 2) Confirm secrets exist

Before first release, verify repository secrets are present:

- `OSSRH_USERNAME`
- `OSSRH_TOKEN`
- `OSSRH_GPG_SECRET_KEY`
- `OSSRH_GPG_SECRET_KEY_PASSWORD`

## 3) Create publish tag

Tag pattern is mandatory:

```bash
git tag sdk-java-v0.4.1
git push origin sdk-java-v0.4.1
```

This triggers `publish-java-sdk.yml`.

## 4) Monitor workflow

Use GitHub Actions UI or CLI:

```bash
gh run list --workflow publish-java-sdk.yml --limit 1
gh run view <run-id> --log
```

## 5) Verify publication

After successful workflow, verify in Central search and dependency resolve in a sample project.

Minimal Maven dependency:

```xml
<dependency>
  <groupId>io.edgequake</groupId>
  <artifactId>edgequake-sdk</artifactId>
  <version>0.4.1</version>
</dependency>
```

## 6) Roll-forward strategy

If publish fails after tag:

1. fix root cause
2. bump version if needed
3. create new tag (`sdk-java-vX.Y.Z+1`)
4. re-run publish

Avoid reusing a failed version if artifacts were partially staged.

