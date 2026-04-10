# OODA-28 Orient

safety_limits.rs has zero tests despite containing critical config clamping and API key validation logic. Pure functions are easily testable: SafetyLimitsConfig::new, ::strict, ::permissive, check_api_key (local providers pass), default_model_for_provider.
