//! SPEC-163 SSRF / egress checks for user-supplied provider URLs.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SsrfError {
    #[error("invalid provider URL: {0}")]
    InvalidUrl(String),
    #[error("provider URL must be http or https")]
    BadScheme,
    #[error("provider URL host is blocked ({0})")]
    BlockedHost(String),
    #[error("provider URL resolved to a blocked address ({0})")]
    BlockedIp(String),
    #[error("private or loopback URLs require locality=local (set allow_private_network)")]
    PrivateDenied,
    #[error("provider host did not resolve ({0})")]
    DnsFailed(String),
}

#[derive(Debug, Clone, Copy)]
pub struct SsrfPolicy {
    /// Allow loopback / RFC1918 / ULA when the connection is marked local.
    pub allow_private: bool,
}

impl Default for SsrfPolicy {
    fn default() -> Self {
        Self {
            allow_private: std::env::var("EDGEQUAKE_DEV_MODE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
        }
    }
}

pub fn validate_provider_url(raw: &str, policy: SsrfPolicy) -> Result<Url, SsrfError> {
    let url = Url::parse(raw).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(SsrfError::BadScheme);
    }
    let host = url
        .host_str()
        .ok_or_else(|| SsrfError::InvalidUrl("missing host".into()))?;
    if is_blocked_hostname(host) || is_obfuscated_ip_host(host) {
        return Err(SsrfError::BlockedHost(host.to_string()));
    }
    if let Some(ip) = parse_literal_ip(host) {
        classify_ip(ip, policy)?;
    }
    Ok(url)
}

/// Resolve a non-literal host and reject metadata, link-local, and (unless opted in) private addresses.
///
/// Literal IPs are already classified by [`validate_provider_url`]. A failed lookup does not
/// connect. This is check-then-connect: a rebinding race after the lookup is still possible.
pub async fn enforce_resolved_addresses(raw: &str, policy: SsrfPolicy) -> Result<(), SsrfError> {
    let url = Url::parse(raw).map_err(|e| SsrfError::InvalidUrl(e.to_string()))?;
    let host = url
        .host_str()
        .ok_or_else(|| SsrfError::InvalidUrl("missing host".into()))?;
    if parse_literal_ip(host).is_some() {
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(80);
    let lookup_host = host.trim_matches(|c| c == '[' || c == ']').to_string();
    let addrs = tokio::net::lookup_host((lookup_host.as_str(), port))
        .await
        .map_err(|_| SsrfError::DnsFailed(host.to_string()))?;
    let mut saw = false;
    for addr in addrs {
        saw = true;
        validate_resolved_ip(addr.ip(), policy)?;
    }
    if !saw {
        return Err(SsrfError::DnsFailed(host.to_string()));
    }
    Ok(())
}

/// Re-check after DNS (anti-rebinding). Call with each resolved address.
pub fn validate_resolved_ip(ip: IpAddr, policy: SsrfPolicy) -> Result<(), SsrfError> {
    classify_ip(ip, policy)
}

fn parse_literal_ip(host: &str) -> Option<IpAddr> {
    let trimmed = host.trim_matches(|c| c == '[' || c == ']');
    trimmed.parse().ok()
}

/// Decimal (`2130706433`), hex (`0x7f000001`), and dotted-octal (`0177.0.0.1`) hosts.
fn is_obfuscated_ip_host(host: &str) -> bool {
    let h = host.trim_matches(|c| c == '[' || c == ']');
    if !h.is_empty() && h.chars().all(|c| c.is_ascii_digit()) {
        return true;
    }
    let lower = h.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("0x") {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit()) {
            return true;
        }
    }
    h.split('.').any(|part| {
        part.len() > 1 && part.starts_with('0') && part.chars().all(|c| c.is_ascii_digit())
    })
}

fn is_blocked_hostname(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    matches!(
        h.as_str(),
        "metadata.google.internal"
            | "metadata.google.internal."
            | "kubernetes.default"
            | "kubernetes.default.svc"
            | "instance-data"
    ) || h.ends_with(".internal")
        || h == "metadata"
        || h.starts_with("169.254.")
}

fn classify_ip(ip: IpAddr, policy: SsrfPolicy) -> Result<(), SsrfError> {
    if is_metadata_or_link_local(ip) {
        return Err(SsrfError::BlockedIp(ip.to_string()));
    }
    if is_private_or_loopback(ip) && !policy.allow_private {
        return Err(SsrfError::PrivateDenied);
    }
    Ok(())
}

pub fn is_metadata_or_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_link_local()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
                || v4 == Ipv4Addr::new(169, 254, 169, 254)
        }
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            // fe80::/10 link-local
            (segs[0] & 0xffc0) == 0xfe80
                // IPv4-mapped 169.254.0.0/16
                || v6.to_ipv4_mapped().is_some_and(|v4| is_metadata_or_link_local(IpAddr::V4(v4)))
        }
    }
}

pub fn is_private_or_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_unspecified(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unique_local()
                || v6.is_unspecified()
                || v6 == Ipv6Addr::LOCALHOST
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_metadata_and_odd_encodings() {
        let deny = SsrfPolicy {
            allow_private: true,
        };
        assert!(validate_provider_url("http://169.254.169.254/latest", deny).is_err());
        // 2852039166 is 169.254.169.254. The URL parser may normalize it to an IP.
        assert!(validate_provider_url("http://2852039166/", deny).is_err());
        let cloud = SsrfPolicy {
            allow_private: false,
        };
        // 2130706433 is 127.0.0.1; 0x7f000001 and 0177.0.0.1 are the same loopback.
        assert!(validate_provider_url("http://2130706433/", cloud).is_err());
        assert!(validate_provider_url("http://0x7f000001/", cloud).is_err());
        assert!(validate_provider_url("http://0177.0.0.1/", cloud).is_err());
        assert!(validate_provider_url("http://metadata.google.internal/", deny).is_err());
        assert!(validate_provider_url("ftp://example.com", deny).is_err());
        assert!(validate_resolved_ip("169.254.169.254".parse().unwrap(), deny).is_err());
        let mapped: IpAddr = "::ffff:169.254.169.254".parse().unwrap();
        assert!(validate_resolved_ip(mapped, deny).is_err());
    }

    #[test]
    fn private_requires_opt_in() {
        let cloud = SsrfPolicy {
            allow_private: false,
        };
        assert!(matches!(
            validate_provider_url("http://127.0.0.1:9050", cloud),
            Err(SsrfError::PrivateDenied)
        ));
        let local = SsrfPolicy {
            allow_private: true,
        };
        assert!(validate_provider_url("http://127.0.0.1:9050", local).is_ok());
        assert!(validate_provider_url("https://api.openai.com/v1", cloud).is_ok());
    }

    #[tokio::test]
    async fn localhost_name_is_private_after_dns() {
        let cloud = SsrfPolicy {
            allow_private: false,
        };
        let err = enforce_resolved_addresses("http://localhost:9", cloud)
            .await
            .expect_err("localhost resolves to loopback");
        assert!(
            matches!(err, SsrfError::PrivateDenied | SsrfError::BlockedIp(_)),
            "{err:?}"
        );
        let local = SsrfPolicy {
            allow_private: true,
        };
        assert!(enforce_resolved_addresses("http://localhost:9", local)
            .await
            .is_ok());
    }
}
