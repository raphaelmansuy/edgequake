//! Live OTLP/HTTP round-trip against Langfuse ≥ 3.22 (self-hosted pin or Cloud).
//!
//! Default `cargo test` no-ops unless `LANGFUSE_OTLP_E2E=1`. Missing base/keys
//! then panic — this is the unfakable gate, not a skip.

use std::time::Duration;

use opentelemetry::trace::{TraceContextExt, Tracer, TracerProvider as _};
use opentelemetry::KeyValue;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use serde_json::Value as JsonValue;

use crate::langfuse::{
    basic_auth_token, langfuse_otlp_headers, probe_langfuse_api, LangfuseApi, LangfuseConfig,
};
use crate::langfuse_attrs::{
    GEN_AI_USAGE_INPUT_TOKENS, GEN_AI_USAGE_OUTPUT_TOKENS, LANGFUSE_OBSERVATION_INPUT,
    LANGFUSE_OBSERVATION_TYPE, OBSERVATION_TYPE_GENERATION, OBSERVATION_TYPE_RETRIEVER,
};

/// Parse `3.22.0` / `4.22.0-rc.1` into (major, minor, patch).
pub(crate) fn parse_langfuse_version(ver: &str) -> Option<(u32, u32, u32)> {
    let core = ver.split(['-', '+']).next().unwrap_or(ver).trim();
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().unwrap_or(0);
    Some((major, minor, patch))
}

pub(crate) fn version_ge(ver: &str, min: (u32, u32, u32)) -> bool {
    parse_langfuse_version(ver).is_some_and(|v| v >= min)
}

/// Exact major.minor pin (`3.22.0` matches 3.22.x, not 3.225.x).
pub(crate) fn pin_major_minor(ver: &str, major: u32, minor: u32) -> bool {
    parse_langfuse_version(ver).is_some_and(|v| v.0 == major && v.1 == minor)
}

/// Known Langfuse Cloud hosts (EU / US / JP / HIPAA). Not `*.langfuse.com`.
pub(crate) fn is_langfuse_cloud_base(base: &str) -> bool {
    let host = host_of(base);
    matches!(
        host.as_str(),
        "cloud.langfuse.com"
            | "us.cloud.langfuse.com"
            | "jp.cloud.langfuse.com"
            | "hipaa.cloud.langfuse.com"
    )
}

fn host_of(base: &str) -> String {
    let s = base.trim().trim_end_matches('/');
    let after_scheme = s.split("://").nth(1).unwrap_or(s);
    after_scheme
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn json_contains_str(value: &JsonValue, needle: &str) -> bool {
    match value {
        JsonValue::String(s) => s.contains(needle),
        JsonValue::Array(items) => items.iter().any(|v| json_contains_str(v, needle)),
        JsonValue::Object(map) => map.values().any(|v| json_contains_str(v, needle)),
        _ => false,
    }
}

fn urlencoding_name(name: &str) -> String {
    name.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "%20".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[test]
fn langfuse_version_otlp_floor_and_pin() {
    assert!(version_ge("3.22.0", (3, 22, 0)));
    assert!(version_ge("3.22.1", (3, 22, 0)));
    assert!(version_ge("4.22.0", (3, 22, 0)));
    assert!(!version_ge("3.1.1", (3, 22, 0)));
    assert!(!version_ge("3.21.9", (3, 22, 0)));
    assert!(pin_major_minor("3.22.0", 3, 22));
    assert!(pin_major_minor("3.22.1", 3, 22));
    assert!(
        !pin_major_minor("3.225.5", 3, 22),
        "3.225 must not count as 3.22"
    );
    assert!(!pin_major_minor("3.1.1", 3, 22));
}

#[test]
fn langfuse_cloud_hosts_are_explicit() {
    assert!(is_langfuse_cloud_base("https://cloud.langfuse.com/"));
    assert!(is_langfuse_cloud_base("https://us.cloud.langfuse.com"));
    assert!(is_langfuse_cloud_base("https://jp.cloud.langfuse.com"));
    assert!(is_langfuse_cloud_base("https://hipaa.cloud.langfuse.com"));
    assert!(!is_langfuse_cloud_base("http://localhost:3330"));
    assert!(!is_langfuse_cloud_base("https://langfuse.example.com"));
}

/// Fail-closed live OTLP persist. Gated on `LANGFUSE_OTLP_E2E=1`.
///
/// Env:
/// - `LANGFUSE_OTLP_E2E_BASE` or `LANGFUSE_BASE_URL`
/// - `LANGFUSE_PUBLIC_KEY` / `LANGFUSE_SECRET_KEY`
/// - `LANGFUSE_OTLP_E2E_PIN=3.22` — require health.version major.minor == 3.22
/// - `LANGFUSE_OTLP_E2E_MIN=3.22.0` — require health.version ≥ min (default)
/// - `LANGFUSE_OTLP_E2E_CLOUD=1` — require a known Cloud host
#[test]
fn live_langfuse_otlp_roundtrip() {
    if std::env::var("LANGFUSE_OTLP_E2E").ok().as_deref() != Some("1") {
        return;
    }
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let base = std::env::var("LANGFUSE_OTLP_E2E_BASE")
        .or_else(|_| std::env::var("LANGFUSE_BASE_URL"))
        .expect("LANGFUSE_OTLP_E2E=1 requires LANGFUSE_OTLP_E2E_BASE or LANGFUSE_BASE_URL");
    let pk = std::env::var("LANGFUSE_PUBLIC_KEY").expect("LANGFUSE_PUBLIC_KEY");
    let sk = std::env::var("LANGFUSE_SECRET_KEY").expect("LANGFUSE_SECRET_KEY");
    let base = crate::langfuse::normalize_base_url(&base);
    let pk = crate::langfuse::unquote_env_value(&pk);
    let sk = crate::langfuse::unquote_env_value(&sk);
    assert!(
        !pk.is_empty() && !sk.is_empty(),
        "Langfuse keys must be set"
    );

    if std::env::var("LANGFUSE_OTLP_E2E_CLOUD").ok().as_deref() == Some("1") {
        assert!(
            is_langfuse_cloud_base(&base),
            "LANGFUSE_OTLP_E2E_CLOUD=1 requires a Langfuse Cloud host, got {base}"
        );
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("client");
    let token = basic_auth_token(&pk, &sk);

    let health: JsonValue = client
        .get(format!("{base}/api/public/health"))
        .send()
        .expect("health HTTP")
        .error_for_status()
        .expect("health status")
        .json()
        .expect("health json");
    let ver = health
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(!ver.is_empty(), "Langfuse health.version must be non-empty");
    eprintln!("live Langfuse health.version={ver} base={base}");

    if let Ok(pin) = std::env::var("LANGFUSE_OTLP_E2E_PIN") {
        let pin = pin.trim();
        let (maj, min) = match pin.split('.').collect::<Vec<_>>().as_slice() {
            [m, n, ..] => (
                m.parse::<u32>().expect("pin major"),
                n.parse::<u32>().expect("pin minor"),
            ),
            _ => panic!("LANGFUSE_OTLP_E2E_PIN must be major.minor (got {pin:?})"),
        };
        assert!(
            pin_major_minor(&ver, maj, min),
            "unfakable pin {pin}: health.version={ver:?}"
        );
    }

    let min_raw = std::env::var("LANGFUSE_OTLP_E2E_MIN").unwrap_or_else(|_| "3.22.0".into());
    let min = parse_langfuse_version(&min_raw).expect("LANGFUSE_OTLP_E2E_MIN semver");
    assert!(
        version_ge(&ver, min),
        "OTLP requires Langfuse ≥ {min_raw}, got version={ver:?}"
    );

    let otlp = client
        .post(format!("{base}/api/public/otel/v1/traces"))
        .header("Authorization", format!("Basic {token}"))
        .header("Content-Type", "application/x-protobuf")
        .body(Vec::<u8>::new())
        .send()
        .expect("otlp probe");
    let otlp_code = otlp.status().as_u16();
    assert_ne!(
        otlp_code, 404,
        "OTLP must exist on ≥ 3.22 / Cloud (got HTTP {otlp_code})"
    );

    assert_eq!(
        probe_langfuse_api(&base, &token),
        LangfuseApi::Otlp,
        "auto-probe must resolve to OTLP on {base} version={ver}"
    );

    let marker = format!("eq-otlp-{}", uuid::Uuid::new_v4());
    let cfg = LangfuseConfig {
        enabled: true,
        base_url: base.clone(),
        public_key_configured: true,
        secret_key_configured: true,
        ui_url: base.clone(),
    };
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(cfg.otlp_endpoint())
        .with_headers(langfuse_otlp_headers(&pk, &sk))
        .build()
        .expect("OTLP SpanExporter");
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name("edgequake-otlp-e2e")
                .build(),
        )
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("edgequake-otlp-e2e");
    tracer.in_span("generate-answer", |cx| {
        let span = cx.span();
        span.set_attributes([
            KeyValue::new(LANGFUSE_OBSERVATION_TYPE, OBSERVATION_TYPE_GENERATION),
            KeyValue::new("gen_ai.request.model", "gpt-5-nano"),
            KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, 3i64),
            KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, 1i64),
            KeyValue::new(LANGFUSE_OBSERVATION_INPUT, marker.clone()),
        ]);
    });
    tracer.in_span("retrieval edgequake", |cx| {
        let span = cx.span();
        span.set_attributes([
            KeyValue::new(LANGFUSE_OBSERVATION_TYPE, OBSERVATION_TYPE_RETRIEVER),
            KeyValue::new(LANGFUSE_OBSERVATION_INPUT, marker.clone()),
        ]);
    });
    provider.force_flush().expect("OTLP force_flush");
    provider.shutdown().expect("OTLP shutdown");

    if std::env::var("LANGFUSE_OTLP_E2E_PERSIST").ok().as_deref() == Some("0") {
        eprintln!(
            "LANGFUSE_OTLP_E2E_PERSIST=0: route+probe only (3.22.0 first-release OTLP parser)"
        );
        return;
    }

    let poll_rounds: u32 = std::env::var("LANGFUSE_OTLP_E2E_POLL_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(45);
    let mut found = false;
    for _ in 0..poll_rounds {
        std::thread::sleep(Duration::from_secs(2));
        let urls = [
            format!(
                "{base}/api/public/observations?limit=50&name={}",
                urlencoding_name("generate-answer")
            ),
            format!(
                "{base}/api/public/observations?limit=50&name={}",
                urlencoding_name("retrieval edgequake")
            ),
            format!("{base}/api/public/observations?limit=50"),
            format!("{base}/api/public/v2/observations?limit=50"),
            format!("{base}/api/public/traces?limit=50"),
        ];
        for url in urls {
            let Ok(resp) = client
                .get(&url)
                .header("Authorization", format!("Basic {token}"))
                .send()
            else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(body) = resp.json::<JsonValue>() else {
                continue;
            };
            if json_contains_str(&body, &marker) {
                found = true;
                break;
            }
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "Langfuse {ver} at {base} did not persist OTLP spans (marker={marker})"
    );
}

/// SPEC-145 E-145-01: live OTLP must persist Complete I/O past the old 512-byte cap.
///
/// Gated on `LANGFUSE_SPEC145_E2E=1` (same keys/base as OTLP e2e).
#[test]
fn live_spec145_complete_io_roundtrip() {
    if std::env::var("LANGFUSE_SPEC145_E2E").ok().as_deref() != Some("1") {
        return;
    }
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    use crate::io_policy::MARKER_TAIL_COMPLETE;
    use crate::langfuse_attrs::LANGFUSE_OBSERVATION_OUTPUT;
    use crate::rag_span::record_observation_io;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let base = std::env::var("LANGFUSE_OTLP_E2E_BASE")
        .or_else(|_| std::env::var("LANGFUSE_BASE_URL"))
        .expect("LANGFUSE_SPEC145_E2E=1 requires LANGFUSE_BASE_URL");
    let pk = std::env::var("LANGFUSE_PUBLIC_KEY").expect("LANGFUSE_PUBLIC_KEY");
    let sk = std::env::var("LANGFUSE_SECRET_KEY").expect("LANGFUSE_SECRET_KEY");
    let base = crate::langfuse::normalize_base_url(&base);
    let pk = crate::langfuse::unquote_env_value(&pk);
    let sk = crate::langfuse::unquote_env_value(&sk);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("client");
    let token = basic_auth_token(&pk, &sk);

    let unique = format!("eq145-{}", uuid::Uuid::new_v4());
    let mut output = "a".repeat(600);
    output.push_str(MARKER_TAIL_COMPLETE);
    output.push('_');
    output.push_str(&unique);

    let cfg = LangfuseConfig {
        enabled: true,
        base_url: base.clone(),
        public_key_configured: true,
        secret_key_configured: true,
        ui_url: base.clone(),
    };
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(cfg.otlp_endpoint())
        .with_headers(langfuse_otlp_headers(&pk, &sk))
        .build()
        .expect("OTLP SpanExporter");
    let provider = SdkTracerProvider::builder()
        .with_resource(
            Resource::builder()
                .with_service_name("edgequake-spec145-e2e")
                .build(),
        )
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("edgequake-spec145-e2e");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let _guard = tracing_subscriber::registry()
        .with(otel_layer)
        .set_default();

    tracing::info_span!(
        "generate-answer",
        otel.name = "generate-answer",
        langfuse.observation.type = OBSERVATION_TYPE_GENERATION,
        langfuse.observation.input = tracing::field::Empty,
        langfuse.observation.output = tracing::field::Empty,
        gen_ai.prompt = tracing::field::Empty,
        gen_ai.completion = tracing::field::Empty,
    )
    .in_scope(|| {
        record_observation_io(Some("SYNTH_ORG query"), Some(&output));
    });

    provider.force_flush().expect("flush");
    provider.shutdown().expect("shutdown");

    let poll_rounds: u32 = std::env::var("LANGFUSE_OTLP_E2E_POLL_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(45);
    let mut found = false;
    for _ in 0..poll_rounds {
        std::thread::sleep(Duration::from_secs(2));
        let urls = [
            format!(
                "{base}/api/public/observations?limit=50&name={}",
                urlencoding_name("generate-answer")
            ),
            format!("{base}/api/public/v2/observations?limit=50"),
            format!("{base}/api/public/observations?limit=50"),
        ];
        for url in urls {
            let Ok(resp) = client
                .get(&url)
                .header("Authorization", format!("Basic {token}"))
                .send()
            else {
                continue;
            };
            if !resp.status().is_success() {
                continue;
            }
            let Ok(body) = resp.json::<JsonValue>() else {
                continue;
            };
            if json_contains_str(&body, MARKER_TAIL_COMPLETE) && json_contains_str(&body, &unique) {
                // Ensure we did not only get a truncated prefix without the unique tail.
                found = true;
                break;
            }
            let _ = LANGFUSE_OBSERVATION_OUTPUT;
        }
        if found {
            break;
        }
    }
    assert!(
        found,
        "Langfuse at {base} missing Complete I/O tail (marker={MARKER_TAIL_COMPLETE} unique={unique})"
    );
}
