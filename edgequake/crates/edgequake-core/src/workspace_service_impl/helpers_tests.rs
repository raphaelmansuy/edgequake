#[cfg(feature = "postgres")]
mod tests {
    use super::super::helpers::normalize_entity_types;

    #[test]
    fn normalize_entity_types_trims_dedupes_and_caps() {
        let input = vec![
            " person ".to_string(),
            "PERSON".to_string(),
            "org-unit".to_string(),
            "".to_string(),
            "   ".to_string(),
        ];
        let out = normalize_entity_types(&input);
        assert_eq!(out, vec!["PERSON".to_string(), "ORG_UNIT".to_string()]);
    }

    #[test]
    fn normalize_entity_types_respects_max_fifty() {
        let input: Vec<String> = (0..60).map(|i| format!("type_{i}")).collect();
        assert_eq!(normalize_entity_types(&input).len(), 50);
    }
}
