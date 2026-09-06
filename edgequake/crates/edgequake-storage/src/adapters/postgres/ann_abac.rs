//! SPEC-146 M2 — ANN over-fetch + allow-set helpers for typed indexes.

use uuid::Uuid;

/// Over-fetch factor when document allow-set filters HNSW results (LAW-146-21).
pub const ABAC_ANN_OVERFETCH_FACTOR: u32 = 4;
/// Cap on over-fetched limit (before truncating to caller `top_k`).
pub const ABAC_ANN_OVERFETCH_CAP: u32 = 200;
/// Above this cardinality, use UNNEST instead of `= ANY($n)` (LAW-146-21).
pub const ALLOW_SET_ARRAY_THRESHOLD: usize = 2048;

/// `min(top_k * factor, cap)` floored at `top_k`.
pub fn abac_overfetch_limit(top_k: u32) -> u32 {
    if top_k == 0 {
        return 0;
    }
    top_k
        .saturating_mul(ABAC_ANN_OVERFETCH_FACTOR)
        .min(ABAC_ANN_OVERFETCH_CAP)
        .max(top_k)
}

/// SQL predicate: `col = ANY($n)` below threshold, `col IN (SELECT unnest($n::uuid[]))` above.
pub fn document_id_allow_sql(column: &str, param: usize, allow_len: usize) -> String {
    uuid_in_allow_sql(column, &format!("${param}"), allow_len)
}

/// Same switch with a placeholder such as `$ALLOW` or `$4`.
pub fn uuid_in_allow_sql(expr: &str, param: &str, allow_len: usize) -> String {
    if allow_len > ALLOW_SET_ARRAY_THRESHOLD {
        format!("{expr} IN (SELECT unnest({param}::uuid[]))")
    } else {
        format!("{expr} = ANY({param}::uuid[])")
    }
}

/// Parse metadata document id strings into UUIDs (invalid ids dropped).
pub fn parse_allow_uuids(ids: Option<&[String]>) -> Option<Vec<Uuid>> {
    ids.map(|list| {
        list.iter()
            .filter_map(|s| Uuid::parse_str(s.trim()).ok())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overfetch_respects_cap_and_floor() {
        assert_eq!(abac_overfetch_limit(10), 40);
        assert_eq!(abac_overfetch_limit(100), 200);
        assert_eq!(abac_overfetch_limit(0), 0);
    }

    #[test]
    fn spec146_parse_allow_uuids_drops_invalid() {
        let ids = vec![
            Uuid::nil().to_string(),
            "not-a-uuid".into(),
            " ".into(),
        ];
        let parsed = parse_allow_uuids(Some(&ids)).expect("some");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], Uuid::nil());
        assert!(parse_allow_uuids(None).is_none());
    }

    #[test]
    fn spec146_allow_sql_switches_at_threshold() {
        let small = document_id_allow_sql("c.document_id", 4, 10);
        assert!(small.contains("= ANY($4::uuid[])"), "{small}");
        let large = document_id_allow_sql("c.document_id", 4, ALLOW_SET_ARRAY_THRESHOLD + 1);
        assert!(large.contains("unnest($4::uuid[])"), "{large}");
    }
}
