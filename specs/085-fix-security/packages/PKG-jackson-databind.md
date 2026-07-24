# `PKG-jackson-databind` — PTV / SSRF / view bypasses

> **Priority**: P1  
> **Audit status**: OPEN  
> **Wave**: 4  
> **Laws**: LAW-15, LAW-16, LAW-20  
> **Dependabot**: #337–#344, #384–#389  
> **Verified against**: v0.21.1 / 2026-07-24

---

## 1. WHY

**Class**: S-sdk. Java/Kotlin SDKs deserialize API JSON with Jackson. High-severity PolymorphicTypeValidator bypasses and SSRF via `InetSocketAddress` matter if consumers enable polymorphic typing or deserialize untrusted payloads into sensitive types.

EdgeQuake SDKs pin **`${jackson.version}=2.18.3`** in both POMs — below floors 2.18.8 / **2.18.9**.

---

## 2. Advisories

| GHSA | Sev | Patched |
|------|-----|---------|
| [GHSA-j3rv-43j4-c7qm](https://github.com/advisories/GHSA-j3rv-43j4-c7qm) | high | 2.18.8 |
| [GHSA-rmj7-2vxq-3g9f](https://github.com/advisories/GHSA-rmj7-2vxq-3g9f) | high | 2.18.8 |
| [GHSA-hgj6-7826-r7m5](https://github.com/advisories/GHSA-hgj6-7826-r7m5) | medium | 2.18.8 |
| [GHSA-5jmj-h7xm-6q6v](https://github.com/advisories/GHSA-5jmj-h7xm-6q6v) | medium | 2.18.9 |
| [GHSA-3pjw-73gf-8qr5](https://github.com/advisories/GHSA-3pjw-73gf-8qr5) | medium | 2.18.8 |
| [GHSA-5gvw-p9qm-jgwh](https://github.com/advisories/GHSA-5gvw-p9qm-jgwh) | medium | 2.18.9 |
| [GHSA-mhm7-754m-9p8w](https://github.com/advisories/GHSA-mhm7-754m-9p8w) | medium | 2.18.9 |

**Security floor**: **`≥2.18.9`**.

**Maven Central latest (audit day)**: `2.22.1` — do **not** jump majors unless SDK tests demand it (LAW-18). Prefer latest **2.18.x ≥2.18.9**.

---

## 3. Current pins

| File | Property |
|------|----------|
| [`sdks/java/pom.xml`](../../../sdks/java/pom.xml) | `<jackson.version>2.18.3</jackson.version>` |
| [`sdks/kotlin/pom.xml`](../../../sdks/kotlin/pom.xml) | `<jackson.version>2.18.3</jackson.version>` |

---

## 4. Target

| Field | Value |
|-------|-------|
| Target | **`2.18.9`** (or newest 2.18.x) in **both** POMs |
| DRY | Single property name; same value (Liskov across SDKs) |

---

## 5. Upgrade steps

1. Set `<jackson.version>2.18.9</jackson.version>` in java + kotlin.  
2. `mvn -q test` in each SDK.  
3. `mvn dependency:tree -Dincludes=com.fasterxml.jackson.core:jackson-databind`.

---

## 6. Edge cases / residual risk

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Consumer enables default typing | Document: keep PTV strict; SDK should not enable polymorphic default typing |
| EC-2 | Jump to 2.22 | Only if 2.18.x unavailable; full SDK retest |

---

## 7. Verification

| Gate ID | Assertion |
|---------|-----------|
| `sec085_jackson_2189` | both SDKs ≥2.18.9; tests green |

Expected close: **#337–#344, #384–#389**.

---

## 8. Cross-refs

Wave 4 · Register `com.fasterxml.jackson.core:jackson-databind`
