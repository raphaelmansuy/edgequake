//! Load a stored provider connection into a [`ConnectionSpec`].

use uuid::Uuid;

/// Parse a workspace role `connection_id` string.
pub fn parse_connection_id(raw: &str) -> Option<Uuid> {
    Uuid::parse_str(raw.trim()).ok()
}

/// Decrypt a `provider_connections` row into a runtime spec.
#[cfg(feature = "postgres")]
pub async fn load_connection_spec(
    pool: &sqlx::PgPool,
    id: Uuid,
    model: &str,
) -> Result<crate::providers::connection_factory::ConnectionSpec, crate::error::ApiError> {
    use crate::error::ApiError;
    use crate::providers::connection_factory::ConnectionSpec;

    let row: (
        String,
        String,
        String,
        Option<Vec<u8>>,
        Option<Vec<u8>>,
        Option<String>,
    ) = sqlx::query_as(
        r#"SELECT api_shape, base_url, auth_scheme, api_key_ciphertext, api_key_nonce, key_id
           FROM provider_connections WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound("connection not found".into()))?;

    let api_key = match (row.3, row.4, row.5) {
        (Some(ct), Some(nonce), Some(kid)) => {
            let env = edgequake_secrets::Envelope {
                ciphertext: ct,
                nonce,
                key_id: kid,
            };
            edgequake_secrets::decrypt_string(&env)
                .ok()
                .map(|s| s.expose().to_string())
        }
        _ => None,
    };

    Ok(ConnectionSpec {
        shape: row.0,
        base_url: row.1,
        api_key,
        model: model.to_string(),
        embedding_model: None,
        embedding_dimension: None,
    })
}
