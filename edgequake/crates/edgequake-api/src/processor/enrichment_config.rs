#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    pub vlm_base_url: String,
    pub vlm_model: String,
    pub max_pages: usize,
    pub concurrent: usize,
}

impl EnrichmentConfig {
    pub fn from_env() -> Self {
        Self {
            vlm_base_url: std::env::var("ENRICHMENT_VLM_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
            vlm_model: std::env::var("ENRICHMENT_VLM_MODEL")
                .unwrap_or_else(|_| "llava:7b".to_string()),
            max_pages: std::env::var("ENRICHMENT_MAX_PAGES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            // WHY num_cpus / 2: Enrichment is IO-bound (one HTTP call to local VLM).
            // Scaling with CPU count ensures throughput grows on larger machines
            // without manual tuning. Floor at 4 for small machines.
            concurrent: std::env::var("ENRICHMENT_CONCURRENT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| (num_cpus::get() / 2).max(4)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enrichment_config_defaults() {
        std::env::remove_var("ENRICHMENT_VLM_BASE_URL");
        std::env::remove_var("ENRICHMENT_VLM_MODEL");
        std::env::remove_var("ENRICHMENT_MAX_PAGES");
        std::env::remove_var("ENRICHMENT_CONCURRENT");

        let config = EnrichmentConfig::from_env();

        assert_eq!(config.vlm_base_url, "http://localhost:11434/v1");
        assert_eq!(config.vlm_model, "llava:7b");
        assert_eq!(config.max_pages, 5);
        // Default scales with CPU count: (num_cpus / 2).max(4)
        let expected = (num_cpus::get() / 2).max(4);
        assert_eq!(config.concurrent, expected);
    }

    #[test]
    fn test_enrichment_config_from_env() {
        std::env::set_var("ENRICHMENT_VLM_BASE_URL", "http://myhost:8080/v1");
        std::env::set_var("ENRICHMENT_VLM_MODEL", "gemma3:12b");
        std::env::set_var("ENRICHMENT_MAX_PAGES", "3");
        std::env::set_var("ENRICHMENT_CONCURRENT", "8");

        let config = EnrichmentConfig::from_env();

        std::env::remove_var("ENRICHMENT_VLM_BASE_URL");
        std::env::remove_var("ENRICHMENT_VLM_MODEL");
        std::env::remove_var("ENRICHMENT_MAX_PAGES");
        std::env::remove_var("ENRICHMENT_CONCURRENT");

        assert_eq!(config.vlm_base_url, "http://myhost:8080/v1");
        assert_eq!(config.vlm_model, "gemma3:12b");
        assert_eq!(config.max_pages, 3);
        assert_eq!(config.concurrent, 8);
    }
}
