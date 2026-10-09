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
    if is_blocked_hostname(host) {
        return Err(SsrfError::BlockedHost(host.to_string()));
    }
    if let Some(ip) = parse_literal_ip(host) {
        classify_ip(ip, policy)?;
    }
    Ok(url)
}

/// Re-check after DNS (anti-rebinding). Call with each resolved address.
pub fn validate_resolved_ip(ip: IpAddr, policy: SsrfPolicy) -> Result<(), SsrfError> {
    classify_ip(ip, policy)
}

fn parse_literal_ip(host: &str) -> Option<IpAddr> {
    let trimmed = host.trim_matches(|c| c == '[' || c == ']');
    trimmed.parse().ok()
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
}
