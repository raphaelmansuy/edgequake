//! Document ID allocation SSOT (SPEC-042-E E-03).

use sqlx::PgPool;
use uuid::Uuid;

use super::capabilities::{DocumentIdGenerator, PostgresCapabilities};

/// Allocate a new document ID — `uuidv7()` on PG18+, else `Uuid::new_v4()`.
pub async fn allocate_document_id(pool: &PgPool, caps: &PostgresCapabilities) -> String {
    match caps.document_id_generator {
        DocumentIdGenerator::UuidV7 => {
            let id: Option<String> = sqlx::query_scalar("SELECT uuidv7()::text")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();
            id.unwrap_or_else(|| Uuid::new_v4().to_string())
        }
        DocumentIdGenerator::UuidV4 => Uuid::new_v4().to_string(),
    }
}

/// RFC 9562 UUIDv7 version nibble is `7` (first hex digit of the third UUID group).
pub fn is_uuidv7(id: &str) -> bool {
    let normalized = id.trim().to_ascii_lowercase();
    let hex = normalized.strip_prefix("urn:uuid:").unwrap_or(&normalized);
    if hex.len() < 15 {
        return false;
    }
    hex.chars().nth(14) == Some('7')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuidv7_nibble_detection() {
        assert!(is_uuidv7("018f3b5a-7b2c-7890-abcd-ef1234567890"));
        assert!(is_uuidv7("019f2780-cedc-7804-9101-dd7ae363b143"));
        assert!(!is_uuidv7("550e8400-e29b-41d4-a716-446655440000"));
    }
}
