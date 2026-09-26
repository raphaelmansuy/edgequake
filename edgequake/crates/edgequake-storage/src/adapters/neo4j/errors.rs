//! Neo4j Query API error classification.

use edgequake_storage_contracts::AccessError;
use reqwest::StatusCode;

use super::client::Neo4jQueryError;

pub(crate) fn classify_http(status: StatusCode, body: &str) -> AccessError {
    let message = sanitized_body(body);
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            AccessError::ForbiddenScope(format!("Neo4j authentication failed: {message}"))
        }
        StatusCode::TOO_MANY_REQUESTS => {
            AccessError::RateLimited(format!("Neo4j request was throttled: {message}"))
        }
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => {
            AccessError::DeadlineExceeded(format!("Neo4j request timed out: {message}"))
        }
        status if status.is_server_error() => {
            AccessError::Unavailable(format!("Neo4j returned HTTP {status}: {message}"))
        }
        _ => AccessError::Unavailable(format!("Neo4j returned HTTP {status}: {message}")),
    }
}

pub(crate) fn classify_query(errors: &[Neo4jQueryError]) -> AccessError {
    let first = &errors[0];
    let summary = format!("{}: {}", first.code, first.message);
    if first.code.contains(".Constraint")
        || first.code.contains(".AlreadyExists")
        || first.code.contains(".EntityNotFound")
    {
        AccessError::Conflict(summary)
    } else if first.code.contains(".TransientError.")
        || first.code.contains(".Transaction.DeadlockDetected")
    {
        AccessError::SerializationRetry(summary)
    } else if first.code.contains(".Security.") {
        AccessError::ForbiddenScope(summary)
    } else if first.code.contains(".Statement.")
        || first.code.contains(".Request.Invalid")
        || first.code.contains(".ClientError.")
    {
        AccessError::InvalidInput(summary)
    } else {
        AccessError::Unavailable(summary)
    }
}

fn sanitized_body(body: &str) -> String {
    const MAX_ERROR_CHARS: usize = 512;
    let body = body.trim();
    if body.is_empty() {
        return "<empty response>".into();
    }
    let mut chars = body.chars();
    let prefix: String = chars.by_ref().take(MAX_ERROR_CHARS).collect();
    if chars.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_error_is_a_conflict() {
        let error = classify_query(&[Neo4jQueryError {
            code: "Neo.ClientError.Schema.ConstraintValidationFailed".into(),
            message: "duplicate".into(),
        }]);
        assert!(matches!(error, AccessError::Conflict(_)));
    }
}
