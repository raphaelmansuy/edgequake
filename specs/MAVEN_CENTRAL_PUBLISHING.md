# Publishing Java and Kotlin SDKs to Maven Central

This guide explains how to publish the EdgeQuake Java and Kotlin SDKs to Maven Central Repository (via Sonatype OSSRH).

## Prerequisites

### 1. Create Sonatype Account

- Sign up at: https://issues.sonatype.org/secure/RiseUp.jspa
- Create a Jira account if you don't have one
- Request access to the `io.edgequake` groupId by creating a Jira ticket

### 2. Set Up GPG Key

```bash
# Generate a GPG key if you don't have one
gpg --gen-key

# Publish your public key to a keyserver
gpg --keyserver keyserver.ubuntu.com --send-keys <YOUR_KEY_ID>
```

### 3. Configure Maven Settings (~/.m2/settings.xml)

Create or edit `~/.m2/settings.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<settings xmlns="http://maven.apache.org/SETTINGS/1.0.0"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://maven.apache.org/SETTINGS/1.0.0
                      http://maven.apache.org/xsd/settings-1.0.0.xsd">

  <servers>
    <!-- Sonatype OSSRH credentials -->
    <server>
      <id>ossrh</id>
      <username>your-sonatype-username</username>
      <password>your-sonatype-password</password>
    </server>
  </servers>

  <profiles>
    <profile>
      <id>ossrh</id>
      <activation>
        <activeByDefault>true</activeByDefault>
      </activation>
      <properties>
        <gpg.executable>gpg</gpg.executable>
        <gpg.passphrase>your-gpg-passphrase</gpg.passphrase>
        <gpg.defaultKeyname>your-key-id</gpg.defaultKeyname>
      </properties>
    </profile>
  </profiles>
</settings>
```

## Publishing Steps

### Step 1: Update SDK Version

```bash
# For Java SDK
make sdk-java-version VERSION=0.2.0

# For Kotlin SDK
make sdk-kotlin-version VERSION=0.2.0
```

### Step 2: Verify Build

```bash
# For Java SDK
make sdk-java-build

# For Kotlin SDK
make sdk-kotlin-build
```

### Step 3: Publish to Maven Central

```bash
# For Java SDK
make sdk-java-publish

# For Kotlin SDK
make sdk-kotlin-publish
```

The Maven release plugin will:

1. Sign artifacts with your GPG key
2. Upload to Sonatype OSSRH staging repository
3. Automatically release to Maven Central (configured in pom.xml)

## After Publishing

Users can then import the SDK:

### Java

```xml
<dependency>
    <groupId>io.edgequake</groupId>
    <artifactId>edgequake-sdk</artifactId>
    <version>0.2.0</version>
</dependency>
```

### Kotlin

```xml
<dependency>
    <groupId>io.edgequake</groupId>
    <artifactId>edgequake-sdk-kotlin</artifactId>
    <version>0.2.0</version>
</dependency>
```

Or with Gradle:

### Java

```gradle
implementation 'io.edgequake:edgequake-sdk:0.2.0'
```

### Kotlin

```gradle
implementation 'io.edgequake:edgequake-sdk-kotlin:0.2.0'
```

## Troubleshooting

### "Unknown host: s01.oss.sonatype.org"

- Check your internet connection
- Verify Sonatype OSSRH servers are accessible

### "Failed to sign artifacts"

- Verify GPG key is installed: `gpg --list-keys`
- Check your GPG passphrase in settings.xml
- Ensure GPG is installed and in PATH: `which gpg`

### "401 Unauthorized"

- Verify Sonatype username/password in ~/.m2/settings.xml
- Check if credentials are correct in Sonatype Jira

### "Artifact already exists"

- You cannot republish the same version (immutable in Maven Central)
- Bump to a new version (e.g., 0.2.0 → 0.2.1)

## References

- [Sonatype OSSRH Guide](https://central.sonatype.org/publish/publish-guide/)
- [Maven Deployment Documentation](https://maven.apache.org/plugins/maven-deploy-plugin/)
- [GPG Signing Guide](https://maven.apache.org/plugins/maven-gpg-plugin/)
