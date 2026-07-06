//! Configuration resolution chains for explainability (SPEC-043).
//!
//! Single source of truth for `GET /config/effective` and `GET /settings/llm-defaults`.

use edgequake_core::{merge_config_field, ConfigPriorityMode, Workspace};

use crate::handlers::settings::{ConfigLevel, EffectiveConfigResponse};
use crate::server_config_store::ServerConfigSnapshot;

fn non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

fn server_level_label(priority: ConfigPriorityMode) -> String {
    match priority {
        ConfigPriorityMode::ServerFirst => "Server: Database (highest)".to_string(),
        ConfigPriorityMode::EnvFirst => "Server: Database".to_string(),
    }
}

fn server_level_note(priority: ConfigPriorityMode) -> String {
    match priority {
        ConfigPriorityMode::ServerFirst => {
            "Saved via Settings UI. Overrides environment variables when set.".to_string()
        }
        ConfigPriorityMode::EnvFirst => {
            "Saved via Settings UI. Used only when matching env vars are unset.".to_string()
        }
    }
}

fn pick_active_level(
    levels: &[ConfigLevel],
    effective_provider: &str,
    effective_model: &str,
) -> String {
    for lvl in levels.iter().rev() {
        let has_value = lvl.provider.is_some() || lvl.model.is_some();
        if !has_value {
            continue;
        }
        let provider_match = lvl
            .provider
            .as_deref()
            .is_none_or(|p| p == effective_provider);
        let model_match = lvl.model.as_deref().is_none_or(|m| m == effective_model);
        if provider_match && model_match {
            return lvl.level.clone();
        }
    }
    levels
        .last()
        .map(|l| l.level.clone())
        .unwrap_or_else(|| "compiled_default".to_string())
}

/// Build LLM resolution chain including server_config level.
pub fn resolve_llm_chain(snapshot: &ServerConfigSnapshot) -> (Vec<ConfigLevel>, String, String) {
    let compiled_provider = "ollama".to_string();
    let compiled_model = Workspace::default_model_for_provider(&compiled_provider);

    let env_primary_provider = non_empty("EDGEQUAKE_DEFAULT_LLM_PROVIDER");
    let env_primary_model = non_empty("EDGEQUAKE_DEFAULT_LLM_MODEL");
    let env_secondary_provider = non_empty("EDGEQUAKE_LLM_PROVIDER");
    let env_secondary_model = non_empty("EDGEQUAKE_LLM_MODEL");
    let env_alias_provider =
        edgequake_core::env::first_non_empty_env_var(&["MODEL_PROVIDER", "CHAT_PROVIDER"]);
    let env_alias_model =
        edgequake_core::env::first_non_empty_env_var(&["CHAT_MODEL", "LLM_MODEL"]);

    let env_provider = env_primary_provider
        .clone()
        .or_else(|| env_secondary_provider.clone())
        .or_else(|| env_alias_provider.clone());
    let env_model = env_primary_model
        .clone()
        .or_else(|| env_secondary_model.clone())
        .or_else(|| env_alias_model.clone());

    let priority = snapshot.priority_mode;
    let server = &snapshot.llm_defaults;

    let effective_provider = merge_config_field(
        env_provider.clone(),
        server.llm_provider.clone(),
        compiled_provider.clone(),
        priority,
    );
    let effective_model = merge_config_field(
        env_model.clone(),
        server.llm_model.clone(),
        Workspace::default_model_for_provider(&effective_provider),
        priority,
    );

    let server_config_level = ConfigLevel {
        level: "server_config".to_string(),
        label: server_level_label(priority),
        provider: server.llm_provider.clone(),
        model: server.llm_model.clone(),
        active: false,
        note: Some(server_level_note(priority)),
        source: Some("server_config.llm_defaults".to_string()),
    };

    let env_primary_level = ConfigLevel {
        level: "env_primary".to_string(),
        label: "Env: EDGEQUAKE_DEFAULT_LLM_*".to_string(),
        provider: env_primary_provider.clone(),
        model: env_primary_model.clone(),
        active: false,
        note: Some("Recommended primary variables.".to_string()),
        source: Some("EDGEQUAKE_DEFAULT_LLM_PROVIDER | EDGEQUAKE_DEFAULT_LLM_MODEL".to_string()),
    };

    let env_secondary_level = ConfigLevel {
        level: "env_secondary".to_string(),
        label: "Env: EDGEQUAKE_LLM_*".to_string(),
        provider: env_secondary_provider.clone(),
        model: env_secondary_model.clone(),
        active: false,
        note: Some("Single-environment deployment variables.".to_string()),
        source: Some("EDGEQUAKE_LLM_PROVIDER | EDGEQUAKE_LLM_MODEL".to_string()),
    };

    let env_alias_level = ConfigLevel {
        level: "env_alias".to_string(),
        label: "Env: Legacy Aliases".to_string(),
        provider: env_alias_provider.clone(),
        model: env_alias_model.clone(),
        active: false,
        note: Some(
            "Compatibility aliases: MODEL_PROVIDER / CHAT_PROVIDER / CHAT_MODEL / LLM_MODEL"
                .to_string(),
        ),
        source: Some("MODEL_PROVIDER | CHAT_PROVIDER | CHAT_MODEL | LLM_MODEL".to_string()),
    };

    let compiled_level = ConfigLevel {
        level: "compiled_default".to_string(),
        label: "Compiled Default".to_string(),
        provider: Some(compiled_provider),
        model: Some(compiled_model),
        active: false,
        note: Some("Built-in fallback when no env vars or server config are set.".to_string()),
        source: Some("binary constant".to_string()),
    };

    let levels = match priority {
        ConfigPriorityMode::ServerFirst => vec![
            compiled_level,
            env_alias_level,
            env_secondary_level,
            env_primary_level,
            server_config_level,
        ],
        ConfigPriorityMode::EnvFirst => vec![
            compiled_level,
            env_alias_level,
            env_secondary_level,
            server_config_level,
            env_primary_level,
        ],
    };

    let active = pick_active_level(&levels, &effective_provider, &effective_model);
    let levels: Vec<ConfigLevel> = levels
        .into_iter()
        .map(|mut lvl| {
            lvl.active = lvl.level == active;
            lvl
        })
        .collect();

    (levels, effective_provider, effective_model)
}

pub fn resolve_embedding_chain(
    snapshot: &ServerConfigSnapshot,
) -> (Vec<ConfigLevel>, String, String) {
    let compiled_provider = "ollama".to_string();
    let compiled_model = Workspace::default_embedding_model_for_provider(&compiled_provider);

    let env_primary_provider = non_empty("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER");
    let env_primary_model = non_empty("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL");
    let env_secondary_provider = non_empty("EDGEQUAKE_EMBEDDING_PROVIDER");
    let env_secondary_model = non_empty("EDGEQUAKE_EMBEDDING_MODEL");

    let env_provider = env_primary_provider
        .clone()
        .or_else(|| env_secondary_provider.clone());
    let env_model = env_primary_model
        .clone()
        .or_else(|| env_secondary_model.clone());

    let priority = snapshot.priority_mode;
    let server = &snapshot.llm_defaults;

    let effective_provider = merge_config_field(
        env_provider.clone(),
        server.embedding_provider.clone(),
        compiled_provider.clone(),
        priority,
    );
    let effective_model = merge_config_field(
        env_model.clone(),
        server.embedding_model.clone(),
        Workspace::default_embedding_model_for_provider(&effective_provider),
        priority,
    );

    let server_config_level = ConfigLevel {
        level: "server_config".to_string(),
        label: server_level_label(priority),
        provider: server.embedding_provider.clone(),
        model: server.embedding_model.clone(),
        active: false,
        note: Some(server_level_note(priority)),
        source: Some("server_config.llm_defaults".to_string()),
    };

    let env_primary_level = ConfigLevel {
        level: "env_primary".to_string(),
        label: "Env: EDGEQUAKE_DEFAULT_EMBEDDING_*".to_string(),
        provider: env_primary_provider,
        model: env_primary_model,
        active: false,
        note: Some("Recommended primary variables.".to_string()),
        source: Some(
            "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER | EDGEQUAKE_DEFAULT_EMBEDDING_MODEL".to_string(),
        ),
    };

    let env_secondary_level = ConfigLevel {
        level: "env_secondary".to_string(),
        label: "Env: EDGEQUAKE_EMBEDDING_*".to_string(),
        provider: env_secondary_provider,
        model: env_secondary_model,
        active: false,
        note: None,
        source: Some("EDGEQUAKE_EMBEDDING_PROVIDER | EDGEQUAKE_EMBEDDING_MODEL".to_string()),
    };

    let compiled_level = ConfigLevel {
        level: "compiled_default".to_string(),
        label: "Compiled Default".to_string(),
        provider: Some(compiled_provider),
        model: Some(compiled_model),
        active: false,
        note: Some("Built-in embedding fallback.".to_string()),
        source: Some("binary constant".to_string()),
    };

    let levels = match priority {
        ConfigPriorityMode::ServerFirst => vec![
            compiled_level,
            env_secondary_level,
            env_primary_level,
            server_config_level,
        ],
        ConfigPriorityMode::EnvFirst => vec![
            compiled_level,
            env_secondary_level,
            server_config_level,
            env_primary_level,
        ],
    };

    let active = pick_active_level(&levels, &effective_provider, &effective_model);
    let levels: Vec<ConfigLevel> = levels
        .into_iter()
        .map(|mut lvl| {
            lvl.active = lvl.level == active;
            lvl
        })
        .collect();

    (levels, effective_provider, effective_model)
}

pub fn resolve_vision_chain(snapshot: &ServerConfigSnapshot) -> (Vec<ConfigLevel>, String, String) {
    let (llm_levels, llm_effective_provider, llm_effective_model) = resolve_llm_chain(snapshot);
    let compiled_provider = llm_effective_provider.clone();
    let compiled_model = Workspace::default_model_for_provider(&compiled_provider);

    let env_vision_provider = non_empty("EDGEQUAKE_VISION_PROVIDER")
        .or_else(|| non_empty("EDGEQUAKE_VISION_LLM_PROVIDER"));
    let env_vision_model =
        non_empty("EDGEQUAKE_VISION_MODEL").or_else(|| non_empty("EDGEQUAKE_VISION_LLM_MODEL"));

    let vision_provider_source = if non_empty("EDGEQUAKE_VISION_PROVIDER").is_some() {
        "EDGEQUAKE_VISION_PROVIDER"
    } else if non_empty("EDGEQUAKE_VISION_LLM_PROVIDER").is_some() {
        "EDGEQUAKE_VISION_LLM_PROVIDER"
    } else {
        "(inherited from LLM)"
    };
    let vision_model_source = if non_empty("EDGEQUAKE_VISION_MODEL").is_some() {
        "EDGEQUAKE_VISION_MODEL"
    } else if non_empty("EDGEQUAKE_VISION_LLM_MODEL").is_some() {
        "EDGEQUAKE_VISION_LLM_MODEL"
    } else {
        "(inherited from LLM)"
    };

    let priority = snapshot.priority_mode;
    let server = &snapshot.llm_defaults;

    let inherited_provider = llm_effective_provider.clone();
    let inherited_model = llm_effective_model.clone();

    let env_provider = env_vision_provider.clone();
    let env_model = env_vision_model.clone();

    let server_provider = server.vision_provider.clone().or_else(|| {
        if env_vision_provider.is_none() {
            server.llm_provider.clone()
        } else {
            None
        }
    });
    let server_model = server.vision_model.clone().or_else(|| {
        if env_vision_model.is_none() {
            server.llm_model.clone()
        } else {
            None
        }
    });

    let base_provider = merge_config_field(
        env_provider.or_else(|| Some(inherited_provider.clone())),
        server_provider,
        compiled_provider.clone(),
        priority,
    );
    let base_model = merge_config_field(
        env_model.or_else(|| Some(inherited_model.clone())),
        server_model,
        Workspace::default_model_for_provider(&base_provider),
        priority,
    );

    let has_env_vision = env_vision_provider.is_some() || env_vision_model.is_some();
    let has_server_vision = snapshot.llm_defaults.vision_provider.is_some()
        || snapshot.llm_defaults.vision_model.is_some();

    let server_config_level = ConfigLevel {
        level: "server_config".to_string(),
        label: server_level_label(priority),
        provider: snapshot.llm_defaults.vision_provider.clone().or_else(|| {
            if has_env_vision {
                None
            } else {
                snapshot.llm_defaults.llm_provider.clone()
            }
        }),
        model: snapshot.llm_defaults.vision_model.clone().or_else(|| {
            if has_env_vision {
                None
            } else {
                snapshot.llm_defaults.llm_model.clone()
            }
        }),
        active: false,
        note: Some(format!(
            "{} Vision-specific fields inherit LLM server defaults when unset.",
            server_level_note(priority)
        )),
        source: Some("server_config.llm_defaults (vision_* or llm_*)".to_string()),
    };

    let llm_fallback_note = format!(
        "Inherited from LLM config (provider={}, model={}).",
        llm_effective_provider, llm_effective_model
    );

    let levels_base = vec![
        ConfigLevel {
            level: "compiled_default".to_string(),
            label: "Compiled Default (via LLM)".to_string(),
            provider: Some(compiled_provider),
            model: Some(compiled_model),
            active: false,
            note: Some(llm_fallback_note.clone()),
            source: Some("binary constant (LLM default)".to_string()),
        },
        ConfigLevel {
            level: "env_llm_inherit".to_string(),
            label: "Env: Inherited from LLM".to_string(),
            provider: Some(inherited_provider),
            model: Some(inherited_model),
            active: false,
            note: Some(llm_fallback_note),
            source: Some("EDGEQUAKE_DEFAULT_LLM_* | EDGEQUAKE_LLM_* | server_config".to_string()),
        },
        ConfigLevel {
            level: "env_vision".to_string(),
            label: "Env: Vision Override".to_string(),
            provider: env_vision_provider,
            model: env_vision_model,
            active: false,
            note: Some("Dedicated vision env override.".to_string()),
            source: Some(format!(
                "{} | {}",
                vision_provider_source, vision_model_source
            )),
        },
    ];

    let mut levels = levels_base;
    if priority == ConfigPriorityMode::ServerFirst {
        levels.push(server_config_level);
    } else {
        // env-first: server before env_vision in chain order for display
        let env_vision = levels.pop().unwrap();
        levels.push(server_config_level);
        levels.push(env_vision);
    }

    let active = if has_env_vision && priority == ConfigPriorityMode::EnvFirst {
        "env_vision".to_string()
    } else if has_server_vision && priority == ConfigPriorityMode::ServerFirst {
        "server_config".to_string()
    } else if has_env_vision {
        "env_vision".to_string()
    } else {
        llm_levels
            .iter()
            .find(|l| l.active)
            .map(|l| l.level.clone())
            .unwrap_or_else(|| "env_llm_inherit".to_string())
    };

    let levels: Vec<ConfigLevel> = levels
        .into_iter()
        .map(|mut lvl| {
            lvl.active = lvl.level == active;
            lvl
        })
        .collect();

    (levels, base_provider, base_model)
}

pub fn priority_rule(snapshot: &ServerConfigSnapshot) -> String {
    let mode = snapshot.priority_mode.as_str();
    format!(
        "Priority mode: {mode} (EDGEQUAKE_CONFIG_PRIORITY or Settings toggle). \
         Higher-indexed levels override lower. \
         Server-first: compiled_default < env_alias < env_secondary < env_primary < server_config. \
         Env-first: compiled_default < env_alias < env_secondary < server_config < env_primary. \
         Vision inherits from LLM when no vision-specific values are set. \
         Workspace DB and per-request overrides sit above this server-default chain."
    )
}

pub fn build_effective_config(snapshot: &ServerConfigSnapshot) -> EffectiveConfigResponse {
    use crate::handlers::settings::build_config_area;

    let (llm_levels, llm_provider, llm_model) = resolve_llm_chain(snapshot);
    let (emb_levels, emb_provider, emb_model) = resolve_embedding_chain(snapshot);
    let (vis_levels, vis_provider, vis_model) = resolve_vision_chain(snapshot);

    EffectiveConfigResponse {
        llm: build_config_area(llm_levels, llm_provider, llm_model),
        embedding: build_config_area(emb_levels, emb_provider, emb_model),
        vision: build_config_area(vis_levels, vis_provider, vis_model),
        priority_rule: priority_rule(snapshot),
        priority_mode: snapshot.priority_mode.as_str().to_string(),
        server_config_available: snapshot.postgres_available,
    }
}

/// Resolve per-field source labels for GET /settings/llm-defaults.
pub fn resolve_field_sources(
    snapshot: &ServerConfigSnapshot,
) -> std::collections::HashMap<String, String> {
    let (llm_levels, llm_p, llm_m) = resolve_llm_chain(snapshot);
    let (emb_levels, emb_p, emb_m) = resolve_embedding_chain(snapshot);
    let (vis_levels, vis_p, vis_m) = resolve_vision_chain(snapshot);

    let mut map = std::collections::HashMap::new();
    map.insert(
        "llm_provider".into(),
        active_source(&llm_levels, &llm_p, "provider"),
    );
    map.insert(
        "llm_model".into(),
        active_source(&llm_levels, &llm_m, "model"),
    );
    map.insert(
        "embedding_provider".into(),
        active_source(&emb_levels, &emb_p, "provider"),
    );
    map.insert(
        "embedding_model".into(),
        active_source(&emb_levels, &emb_m, "model"),
    );
    map.insert(
        "vision_provider".into(),
        active_source(&vis_levels, &vis_p, "provider"),
    );
    map.insert(
        "vision_model".into(),
        active_source(&vis_levels, &vis_m, "model"),
    );
    map
}

fn active_source(levels: &[ConfigLevel], _value: &str, _field: &str) -> String {
    levels
        .iter()
        .find(|l| l.active)
        .and_then(|l| l.source.clone())
        .unwrap_or_else(|| "compiled_default".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_core::{install_server_config, ConfigPriorityMode, ServerLlmDefaults};

    #[test]
    fn server_first_puts_server_config_at_top_of_chain() {
        let defaults = ServerLlmDefaults {
            llm_provider: Some("openai".into()),
            llm_model: Some("gpt-5-nano".into()),
            ..Default::default()
        };
        install_server_config(defaults.clone(), ConfigPriorityMode::ServerFirst);
        let snapshot = ServerConfigSnapshot {
            llm_defaults: defaults,
            priority_mode: ConfigPriorityMode::ServerFirst,
            app_attribution: Default::default(),
            postgres_available: true,
        };
        let (levels, provider, model) = resolve_llm_chain(&snapshot);
        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-5-nano");
        assert!(levels
            .iter()
            .any(|l| l.level == "server_config" && l.active));
    }
}
