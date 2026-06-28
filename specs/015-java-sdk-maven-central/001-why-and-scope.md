# 001 - Why and Scope

## Why this spec exists

Issue #234 requested public Java SDK distribution so Spring Boot and enterprise Java projects can consume EdgeQuake via normal dependency management (Maven/Gradle), without manual JAR hosting.

Publishing to Maven Central solves:

- standard dependency resolution in Maven/Gradle
- reproducible CI/CD builds in enterprise environments
- semantic versioning and upgrade management
- easier adoption in JVM ecosystems (Spring Boot, Micronaut, Quarkus)

## In scope

- Configure Java SDK package metadata and signing requirements for Central
- Configure GitHub Actions publication workflow to OSSRH
- Define required secrets and how to generate them
- Define release trigger model (`sdk-java-v*` tags)
- Provide operational runbook and troubleshooting

## Out of scope

- Kotlin SDK publication (separate artifact and potentially separate workflow)
- snapshot publication strategy for every commit (optional follow-up)
- Gradle plugin publishing (this is Maven artifact publishing)

## Definition of done

- Java SDK `pom.xml` is Central-ready (sources/javadocs/signing/staging)
- CI workflow can publish signed artifacts to OSSRH on release tags
- Documentation contains end-to-end maintainer process
- Issue #234 is closed with implementation proof

