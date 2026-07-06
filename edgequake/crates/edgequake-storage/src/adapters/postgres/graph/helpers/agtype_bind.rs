//! sqlx binding for Apache AGE `agtype` Cypher parameter maps (SPEC-044).
//!
//! AGE requires the third `cypher()` argument to be a bare `$1` with an
//! **agtype**-typed bind (not `text`). See:
//! <https://age.apache.org/age-manual/master/advanced/prepared_statements.html>
//!
//! Binary wire format: version byte `1` + UTF-8 JSON map text (Npgsql.Age /
//! sqlx `PgLTree` pattern).

use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, Postgres};
use sqlx::{Encode, Type};

use crate::error::{Result, StorageError};

/// JSON-serialized Cypher parameter map encoded as PostgreSQL `agtype`.
#[derive(Debug, Clone)]
pub(in crate::adapters::postgres::graph) struct PgAgtype(pub String);

impl PgAgtype {
    pub(in crate::adapters::postgres::graph) fn from_json(
        params: &serde_json::Value,
    ) -> Result<Self> {
        serde_json::to_string(params)
            .map(Self)
            .map_err(|e| StorageError::Database(format!("Cypher params JSON encode: {e}")))
    }
}

impl Type<Postgres> for PgAgtype {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("agtype")
    }
}

impl Encode<'_, Postgres> for PgAgtype {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> std::result::Result<IsNull, BoxDynError> {
        buf.extend(1i8.to_le_bytes());
        buf.extend(self.0.as_bytes());
        Ok(IsNull::No)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::TypeInfo;

    #[test]
    fn pg_agtype_uses_agtype_sql_type_name() {
        assert_eq!(PgAgtype::type_info().name(), "agtype");
    }
}
