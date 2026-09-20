use edgequake_storage_contracts::{AccessError, AccessResult};
use reqwest::StatusCode;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OperationEnvelope {
    result: Option<OperationResult>,
}

#[derive(Debug, Deserialize)]
struct OperationResult {
    status: String,
    operation_id: Option<u64>,
}

pub(crate) fn transport_error(error: reqwest::Error) -> AccessError {
    if error.is_timeout() {
        AccessError::DeadlineExceeded(format!("Qdrant request timed out: {error}"))
    } else if error.is_connect() {
        AccessError::Unavailable(format!("Qdrant connection failed: {error}"))
    } else {
        AccessError::Unavailable(format!("Qdrant transport failed: {error}"))
    }
}

pub(crate) fn http_error(status: StatusCode, body: &str) -> AccessError {
    let detail = body.trim();
    let message = if detail.is_empty() {
        format!("Qdrant returned HTTP {status}")
    } else {
        format!("Qdrant returned HTTP {status}: {detail}")
    };
    match status {
        StatusCode::TOO_MANY_REQUESTS => AccessError::RateLimited(message),
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => {
            AccessError::DeadlineExceeded(message)
        }
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            AccessError::InvalidInput(message)
        }
        StatusCode::NOT_FOUND => AccessError::NotFound(message),
        _ if status.is_server_error() => AccessError::Unavailable(message),
        _ => AccessError::Unavailable(message),
    }
}

/// A provider update is publishable only after Qdrant reports `completed`.
pub(crate) fn parse_completed_operation(body: &[u8]) -> AccessResult<String> {
    let envelope: OperationEnvelope = serde_json::from_slice(body)
        .map_err(|error| AccessError::CorruptData(format!("invalid Qdrant response: {error}")))?;
    let result = envelope
        .result
        .ok_or_else(|| AccessError::CorruptData("Qdrant response omitted result".into()))?;
    match result.status.as_str() {
        "completed" => Ok(result
            .operation_id
            .map(|id| format!("qdrant-operation:{id}:completed"))
            .unwrap_or_else(|| "qdrant-operation:completed".into())),
        "acknowledged" | "wait_timeout" => Err(AccessError::UnknownOutcome(format!(
            "Qdrant update status was '{}' instead of completed",
            result.status
        ))),
        other => Err(AccessError::CorruptData(format!(
            "unknown Qdrant update status '{other}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_is_the_only_publishable_status() {
        let receipt = parse_completed_operation(
            br#"{"status":"ok","result":{"status":"completed","operation_id":42}}"#,
        )
        .unwrap();
        assert_eq!(receipt, "qdrant-operation:42:completed");

        for status in ["acknowledged", "wait_timeout"] {
            let body = format!(r#"{{"result":{{"status":"{status}"}}}}"#);
            let error = parse_completed_operation(body.as_bytes()).unwrap_err();
            assert!(matches!(error, AccessError::UnknownOutcome(_)));
            assert!(error.is_retryable());
        }
    }

    #[test]
    fn malformed_or_unknown_status_is_corrupt_data() {
        assert!(matches!(
            parse_completed_operation(br#"{"result":{"status":"queued"}}"#),
            Err(AccessError::CorruptData(_))
        ));
        assert!(matches!(
            parse_completed_operation(br#"{"status":"ok"}"#),
            Err(AccessError::CorruptData(_))
        ));
    }
}
