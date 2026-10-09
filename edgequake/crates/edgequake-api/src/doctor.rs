//! `edgequake doctor` — SPEC-163 preflight.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
}

const EXIT_OK: u8 = 0;
const EXIT_WARN: u8 = 2;
const EXIT_FAIL: u8 = 1;

pub fn run_doctor(json: bool) -> i32 {
    let report = collect();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into())
        );
    } else {
        for c in &report.checks {
            let mark = if c.ok { "ok" } else { "FAIL" };
            println!("[{mark}] {}: {}", c.id, c.message);
        }
        println!(
            "{}",
            if report.ok {
                "doctor: all required checks passed"
            } else {
                "doctor: one or more checks failed"
            }
        );
    }
    if report.ok {
        i32::from(EXIT_OK)
    } else if report.checks.iter().any(|c| !c.ok && c.id == "database") {
        i32::from(EXIT_FAIL)
    } else {
        i32::from(EXIT_WARN)
    }
}

pub fn collect() -> DoctorReport {
    let mut checks = Vec::new();

    let db = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
    checks.push(DoctorCheck {
        id: "database".into(),
        ok: db.is_some(),
        message: if db.is_some() {
            "DATABASE_URL is set".into()
        } else {
            "DATABASE_URL is missing".into()
        },
    });

    let provider = std::env::var("EDGEQUAKE_LLM_PROVIDER")
        .or_else(|_| std::env::var("EDGEQUAKE_DEFAULT_LLM_PROVIDER"))
        .unwrap_or_else(|_| "(auto)".into());
    checks.push(DoctorCheck {
        id: "llm_provider".into(),
        ok: true,
        message: format!("default provider: {provider}"),
    });

    checks.push(DoctorCheck {
        id: "secrets_key".into(),
        ok: edgequake_secrets::secrets_configured() || std::env::var("EDGEQUAKE_DEV_MODE").is_ok(),
        message: if edgequake_secrets::secrets_configured() {
            "EDGEQUAKE_SECRETS_KEY configured".into()
        } else {
            "EDGEQUAKE_SECRETS_KEY missing (cannot store connection keys)".into()
        },
    });

    let jwt = std::env::var("JWT_SECRET").unwrap_or_default();
    let jwt_ok = jwt.len() >= 32 && jwt != "change-me-in-production-256-bit-secret-key";
    let dev = std::env::var("EDGEQUAKE_DEV_MODE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    checks.push(DoctorCheck {
        id: "jwt_secret".into(),
        ok: jwt_ok || dev,
        message: if jwt_ok {
            "JWT_SECRET is set".into()
        } else if dev {
            "JWT_SECRET is the default (allowed in EDGEQUAKE_DEV_MODE)".into()
        } else {
            "JWT_SECRET is missing or the public default".into()
        },
    });

    let bind = std::env::var("EDGEQUAKE_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    checks.push(DoctorCheck {
        id: "bind".into(),
        ok: bind == "127.0.0.1" || bind == "localhost" || dev,
        message: format!("listen host {bind}"),
    });

    let ok = checks
        .iter()
        .all(|c| c.ok || matches!(c.id.as_str(), "llm_provider"));
    DoctorReport { ok, checks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_database() {
        std::env::remove_var("DATABASE_URL");
        let r = collect();
        assert!(r.checks.iter().any(|c| c.id == "database" && !c.ok));
    }
}
