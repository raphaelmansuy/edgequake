//! One add-provider preset per non-mock `edgequake_llm::ProviderType`.
//! Aliases stay accepted so existing rows can still be saved.

/// How a provider proves who it is.
///
/// WHY this is explicit and machine-readable rather than prose in `hint`: every provider used to
/// describe its own security in a sentence, and nothing could check it. An operator could save a
/// Vertex connection with an API key that the client refuses to use, and the UI had no way to say
/// so. Modelling the requirement lets the API, the form and the tests agree on one answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRequirement {
    /// A stored API key. The key is a secret; it must never be echoed back.
    ApiKey,
    /// A key plus a non-secret endpoint and deployment name (Azure OpenAI).
    ApiKeyAndEndpoint,
    /// A local or self-hosted server where a key is optional but allowed (vLLM behind auth,
    /// a proxied OpenAI-compatible gateway). Hiding the field here would make a protected local
    /// server impossible to configure, which is why "no key required" is not "no key accepted".
    OptionalApiKey,
    /// Vertex AI. The server identity works with nothing stored. A saved secret is either a
    /// service-account JSON file or a Vertex API key (Express, or a key bound to a service account),
    /// encrypted at rest.
    ServerGoogleIdentity,
    /// The AWS credential chain (environment, profile, instance role).
    AwsChain,
    /// A credential the server already holds through a proxy it controls (VS Code Copilot).
    ServerProxy,
}

impl AuthRequirement {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKey => "api_key",
            Self::ApiKeyAndEndpoint => "api_key_and_endpoint",
            Self::OptionalApiKey => "optional_api_key",
            Self::ServerGoogleIdentity => "server_google_identity",
            Self::AwsChain => "aws_chain",
            Self::ServerProxy => "server_proxy",
        }
    }

    /// Is a key required for this shape to work at all?
    pub fn requires_key(self) -> bool {
        matches!(self, Self::ApiKey | Self::ApiKeyAndEndpoint)
    }

    /// Can this shape be built from a URL and a saved secret?
    ///
    /// Vertex can: a service-account JSON (or a Vertex API key) is encrypted and used to mint
    /// the call. Bedrock and the Copilot proxy still cannot — their clients read a chain the
    /// server already holds, so a pasted value would be stored and then ignored.
    pub fn accepts_saved_key(self) -> bool {
        matches!(
            self,
            Self::ApiKey
                | Self::ApiKeyAndEndpoint
                | Self::OptionalApiKey
                | Self::ServerGoogleIdentity
        )
    }

    /// The server can call this provider with no saved secret (ADC, the AWS chain, a proxy).
    pub fn server_identity_suffices(self) -> bool {
        matches!(
            self,
            Self::ServerGoogleIdentity | Self::AwsChain | Self::ServerProxy
        )
    }

    /// If a key is saved, must it be encrypted and never returned?
    ///
    /// True exactly when a key may be stored. Keeping these two in step is what stops a secret
    /// being written in the clear, or a non-secret being encrypted and then unreadable.
    pub fn stores_secret(self) -> bool {
        self.accepts_saved_key()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionShape {
    pub id: &'static str,
    pub label: &'static str,
    pub default_base_url: &'static str,
    pub hint: &'static str,
    /// What this shape needs to authenticate. Drives the form, the save path and the tests.
    pub auth: AuthRequirement,
}

/// Shapes whose client builds its own URL from the credential or the region.
///
/// Gemini is addressed by model and key (its OpenAI-compat host is a different API), Vertex from
/// project and location, and Bedrock and Copilot from the SDK/proxy. A Base URL field for these
/// looked editable and changed nothing, which is the same class of lie as a key field Vertex
/// ignores. The server states it so the form can disable the field and say why.
pub fn honors_base_url(shape: &str) -> bool {
    !matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "gemini"
            | "google"
            | "vertexai"
            | "vertex"
            | "bedrock"
            | "aws-bedrock"
            | "vscode-copilot"
            | "vscode"
            | "copilot"
    )
}

pub const CONNECTION_SHAPES: &[ConnectionShape] = &[
    ConnectionShape {
        id: "openai",
        label: "OpenAI",
        default_base_url: "https://api.openai.com/v1",
        hint: "API key required.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "anthropic",
        label: "Anthropic",
        default_base_url: "https://api.anthropic.com",
        hint: "API key required. Messages API.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "gemini",
        label: "Gemini",
        default_base_url: "https://generativelanguage.googleapis.com",
        hint: "Google AI Studio API key (GEMINI_API_KEY). A Vertex service-account file belongs on Vertex AI.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "vertexai",
        label: "Vertex AI",
        default_base_url: "https://aiplatform.googleapis.com",
        hint: "Service-account JSON (the key behind gcloud auth activate-service-account), a Vertex Express or service-account-bound API key, or this server's Google Cloud identity. A saved secret is encrypted and never shown again.",
        auth: AuthRequirement::ServerGoogleIdentity,
    },
    ConnectionShape {
        id: "openrouter",
        label: "OpenRouter",
        default_base_url: "https://openrouter.ai/api/v1",
        hint: "API key required. OpenAI-compatible.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "xai",
        label: "xAI",
        default_base_url: "https://api.x.ai/v1",
        hint: "API key required. OpenAI-compatible.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "huggingface",
        label: "Hugging Face",
        // 2026: `api-inference.huggingface.co` was retired (410 Gone). Inference Providers now
        // serve an OpenAI-shaped router. Source: huggingface.co/docs/inference-providers
        default_base_url: "https://router.huggingface.co/v1",
        hint: "Access token required (HF_TOKEN). OpenAI-compatible Inference Providers router.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "openai-compatible",
        label: "OpenAI-compatible",
        default_base_url: "https://api.openai.com/v1",
        hint: "Any server that speaks chat completions. Key optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "ollama",
        label: "Ollama",
        default_base_url: "http://127.0.0.1:11434",
        hint: "Local Ollama. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "lmstudio",
        label: "LM Studio",
        default_base_url: "http://127.0.0.1:1234/v1",
        hint: "Local LM Studio. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "omlx",
        label: "oMLX",
        default_base_url: "http://127.0.0.1:9050/v1",
        hint: "Local oMLX. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "mtplx",
        label: "MTPLX",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "Local MTPLX. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "llamacpp",
        label: "llama.cpp",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "llama-server. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "vllm-mlx",
        label: "vLLM-MLX",
        default_base_url: "http://127.0.0.1:8000/v1",
        hint: "Local vLLM-MLX. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "mlx-lm",
        label: "MLX-LM",
        default_base_url: "http://127.0.0.1:8080/v1",
        hint: "mlx_lm.server. A key is optional.",
        auth: AuthRequirement::OptionalApiKey,
    },
    ConnectionShape {
        id: "vscode-copilot",
        label: "VS Code Copilot",
        default_base_url: "https://api.githubcopilot.com",
        hint: "Uses the Copilot proxy on the server. A saved key cannot build this client.",
        auth: AuthRequirement::ServerProxy,
    },
    ConnectionShape {
        id: "mistral",
        label: "Mistral",
        default_base_url: "https://api.mistral.ai/v1",
        hint: "API key required. OpenAI-compatible.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "azure",
        label: "Azure OpenAI",
        // Foundry v1 route: the resource host plus `/openai/v1`; no dated `api-version`.
        // Source: learn.microsoft.com/azure/foundry/openai/api-version-lifecycle
        default_base_url: "https://YOUR_RESOURCE.openai.azure.com/openai/v1",
        hint: "API key required. The URL must end in /openai/v1 (Foundry v1). The deployment name is the model.",
        auth: AuthRequirement::ApiKeyAndEndpoint,
    },
    ConnectionShape {
        id: "nvidia",
        label: "NVIDIA",
        default_base_url: "https://integrate.api.nvidia.com/v1",
        hint: "API key required. OpenAI-compatible.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "cohere",
        label: "Cohere",
        // `api.cohere.com/v1` is Cohere's *native* chat shape (`message`, not `messages`). A client
        // that speaks OpenAI needs the Compatibility API. Source: docs.cohere.com/docs/compatibility-api
        default_base_url: "https://api.cohere.ai/compatibility/v1",
        hint: "API key required. Uses Cohere's OpenAI-compatible endpoint.",
        auth: AuthRequirement::ApiKey,
    },
    ConnectionShape {
        id: "bedrock",
        label: "AWS Bedrock",
        default_base_url: "https://bedrock-runtime.us-east-1.amazonaws.com",
        hint: "Uses this server's AWS credential chain: an instance role, AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY, or AWS_BEARER_TOKEN_BEDROCK. A key pasted here would be stored and then ignored, so the field is not offered.",
        auth: AuthRequirement::AwsChain,
    },
];

const EXTRA_SHAPES: &[&str] = &[
    "openai_chat",
    "openai_responses",
    "openai_compatible",
    "anthropic_messages",
    "claude",
    "mlx_lm",
    "lm-studio",
    "lm_studio",
    "o-mlx",
    "o_mlx",
    "mtp-lx",
    "llama-server",
    "llama.cpp",
    "vllm_mlx",
    "vertex",
    "grok",
    "hf",
    "huggingface",
    "google",
    "azure-openai",
    "azure_openai",
    "nim",
    "copilot",
    "vscode",
];

/// Unknown shapes are rejected so a typo cannot be stored as a custom provider.
pub fn accept_api_shape(shape: &str) -> bool {
    let normalized = shape.trim().to_ascii_lowercase();
    CONNECTION_SHAPES
        .iter()
        .any(|preset| preset.id == normalized)
        || EXTRA_SHAPES.contains(&normalized.as_str())
}

/// Shapes that speak `{base}/v1/models` and chat completions.
pub fn is_openai_wire(shape: &str) -> bool {
    matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "openai"
            | "openai_chat"
            | "openai_responses"
            | "openai-compatible"
            | "openai_compatible"
            | "openrouter"
            | "mistral"
            | "xai"
            | "grok"
            | "nvidia"
            | "nim"
            | "cohere"
            | "huggingface"
            | "hf"
            | "azure"
            | "azure-openai"
            | "azure_openai"
            | "lmstudio"
            | "lm-studio"
            | "lm_studio"
            | "omlx"
            | "o-mlx"
            | "o_mlx"
            | "mtplx"
            | "mtp-lx"
            | "llamacpp"
            | "llama-server"
            | "llama.cpp"
            | "vllm-mlx"
            | "vllm_mlx"
            | "mlx-lm"
            | "mlx_lm"
    )
}

/// Saved URL and key cannot construct these clients.
pub fn requires_server_credentials(shape: &str) -> bool {
    matches!(
        shape.trim().to_ascii_lowercase().as_str(),
        "bedrock" | "vscode-copilot" | "vscode" | "copilot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgequake_llm::ProviderType;

    #[test]
    fn presets_and_aliases_are_accepted_and_typos_are_not() {
        assert!(accept_api_shape("openai"));
        assert!(accept_api_shape("openai_chat"));
        assert!(accept_api_shape("Anthropic_Messages"));
        assert!(accept_api_shape("lmstudio"));
        assert!(accept_api_shape("openrouter"));
        assert!(accept_api_shape("mistral"));
        assert!(!accept_api_shape(""));
        assert!(!accept_api_shape("custom-http"));
        assert!(!accept_api_shape("openai_chat/../"));
        assert!(!accept_api_shape("mock"));
    }

    #[test]
    fn every_non_mock_provider_type_is_a_connection_shape() {
        let ids: Vec<_> = CONNECTION_SHAPES.iter().map(|shape| shape.id).collect();
        for kind in ProviderType::all() {
            if kind.canonical_id() == "mock" {
                continue;
            }
            assert!(
                ids.contains(&kind.canonical_id()),
                "missing shape {}",
                kind.canonical_id()
            );
            assert!(accept_api_shape(kind.canonical_id()));
        }
    }

    /*
     * SPEC-185 — the security contract, pinned.
     *
     * Each of these was a real way to misconfigure a provider: save a key Vertex cannot use, show
     * a secret field where no secret exists, or treat an identity-only provider as if a key would
     * fix it. The point of the enum is that these are checked rather than described.
     */

    fn shape(id: &str) -> &'static ConnectionShape {
        CONNECTION_SHAPES
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("no shape {id}"))
    }

    #[test]
    fn vertex_ai_stores_a_google_credential_and_still_works_with_the_server_identity() {
        let vertex = shape("vertexai");
        assert_eq!(vertex.auth, AuthRequirement::ServerGoogleIdentity);
        assert!(vertex.auth.accepts_saved_key());
        assert!(vertex.auth.stores_secret());
        assert!(!vertex.auth.requires_key());
        assert!(vertex.auth.server_identity_suffices());
        assert!(!requires_server_credentials("vertexai"));
    }

    #[test]
    fn azure_needs_a_key_and_an_endpoint_and_stores_a_secret() {
        let azure = shape("azure");
        assert_eq!(azure.auth, AuthRequirement::ApiKeyAndEndpoint);
        assert!(azure.auth.accepts_saved_key());
        assert!(azure.auth.stores_secret());
    }

    #[test]
    fn every_cloud_key_provider_stores_a_secret_and_every_local_one_accepts_an_optional_key() {
        for id in [
            "openai",
            "anthropic",
            "gemini",
            "mistral",
            "xai",
            "openrouter",
            "huggingface",
        ] {
            let s = shape(id);
            assert_eq!(s.auth, AuthRequirement::ApiKey, "{id}");
            assert!(s.auth.requires_key(), "{id} cannot work without a key");
            assert!(s.auth.stores_secret(), "{id} must encrypt its key");
        }
        for id in [
            "ollama", "lmstudio", "omlx", "mtplx", "llamacpp", "vllm-mlx", "mlx-lm",
        ] {
            let s = shape(id);
            // "No key required" is not "no key accepted": a protected local server must stay
            // configurable, so the field remains and the value is still a secret.
            assert_eq!(s.auth, AuthRequirement::OptionalApiKey, "{id}");
            assert!(!s.auth.requires_key(), "{id} works without a key");
            assert!(s.auth.accepts_saved_key(), "{id} must still accept one");
            assert!(
                s.auth.stores_secret(),
                "{id} must encrypt a key it does keep"
            );
        }
    }

    #[test]
    fn a_shape_that_accepts_a_key_is_exactly_a_shape_that_stores_one() {
        // If these ever disagree, the save path and the form disagree about whether a value is a
        // secret: one would encrypt a non-secret, or store a secret in the clear.
        for s in CONNECTION_SHAPES {
            assert_eq!(
                s.auth.accepts_saved_key(),
                s.auth.stores_secret(),
                "{} disagrees about keys",
                s.id
            );
        }
    }

    #[test]
    fn a_required_key_is_always_an_accepted_key() {
        for s in CONNECTION_SHAPES {
            if s.auth.requires_key() {
                assert!(
                    s.auth.accepts_saved_key(),
                    "{} requires a key it would refuse",
                    s.id
                );
            }
        }
    }

    #[test]
    fn a_base_url_is_only_offered_where_the_client_reads_it() {
        // Gemini is addressed by model and key, Vertex by project and location, Bedrock and the
        // Copilot proxy by the SDK. A Base URL field for these is a control that changes nothing.
        for id in ["gemini", "google", "vertexai", "vertex", "bedrock"] {
            assert!(!honors_base_url(id), "{id} must not offer a Base URL");
        }
        for id in [
            "openai",
            "anthropic",
            "azure",
            "ollama",
            "openrouter",
            "huggingface",
        ] {
            assert!(honors_base_url(id), "{id} builds from its Base URL");
        }
    }

    #[test]
    fn no_shape_claims_a_key_it_cannot_use() {
        // `requires_server_credentials` is what the factory enforces; the shape's auth must match,
        // otherwise a form offers a key field that the client will reject at save time.
        for s in CONNECTION_SHAPES {
            if requires_server_credentials(s.id) {
                assert!(
                    !s.auth.accepts_saved_key(),
                    "{} accepts a key but cannot build from one",
                    s.id
                );
            }
        }
    }
}
