//! SPEC-163 provider test-connection (list models, chat ping, embed ping).

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use utoipa::ToSchema;

use crate::locality::{default_local_base_url, local_health_path};
use crate::ssrf::{validate_provider_url, SsrfPolicy};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeErrorKind {
    Ok,
    Unreachable,
    Unauthorized,
    ModelNotFound,
    DimMismatch,
    ShapeMismatch,
    SsrfDenied,
    InvalidUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProbeRequest {
    /// `openai_chat`, `anthropic_messages`, `ollama`, or a native provider id.
    pub shape: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub embedding_model: Option<String>,
    pub api_key: Option<String>,
    /// `none`, `bearer`, `x_api_key`
    #[serde(default)]
    pub auth_scheme: Option<String>,
    #[serde(default)]
    pub allow_private_network: Option<bool>,
    /// Expected embedding dimension; mismatch → dim_mismatch.
    pub expected_dimension: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProbeResponse {
    pub ok: bool,
    pub kind: ProbeErrorKind,
    pub latency_ms: u64,
    pub message: String,
    pub models: Vec<String>,
    pub embedding_dimension: Option<usize>,
    pub chat_ok: bool,
    pub embed_ok: bool,
    pub list_ok: bool,
}

pub async fn probe_provider(req: ProbeRequest) -> ProbeResponse {
    let start = Instant::now();
    let shape = req.shape.trim().to_ascii_lowercase();
    let allow_private = req.allow_private_network.unwrap_or_else(|| {
        crate::locality::is_slow_local_provider(&shape)
            || crate::locality::is_local_provider(&shape)
    });
    let base = req
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .or_else(|| default_local_base_url(&shape).map(|s| s.to_string()));

    let Some(base) = base else {
        return ProbeResponse {
            ok: false,
            kind: ProbeErrorKind::InvalidUrl,
            latency_ms: start.elapsed().as_millis() as u64,
            message: "base_url is required for this shape".into(),
            models: vec![],
            embedding_dimension: None,
            chat_ok: false,
            embed_ok: false,
            list_ok: false,
        };
    };

    match validate_provider_url(&base, SsrfPolicy { allow_private }) {
        Ok(_) => {}
        Err(e) => {
            return ProbeResponse {
                ok: false,
                kind: ProbeErrorKind::SsrfDenied,
                latency_ms: start.elapsed().as_millis() as u64,
                message: e.to_string(),
                models: vec![],
                embedding_dimension: None,
                chat_ok: false,
                embed_ok: false,
                list_ok: false,
            };
        }
    }
    if let Err(e) =
        crate::ssrf::enforce_resolved_addresses(&base, SsrfPolicy { allow_private }).await
    {
        let kind = match e {
            crate::ssrf::SsrfError::DnsFailed(_) => ProbeErrorKind::Unreachable,
            _ => ProbeErrorKind::SsrfDenied,
        };
        return ProbeResponse {
            ok: false,
            kind,
            latency_ms: start.elapsed().as_millis() as u64,
            message: e.to_string(),
            models: vec![],
            embedding_dimension: None,
            chat_ok: false,
            embed_ok: false,
            list_ok: false,
        };
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return fail(start, ProbeErrorKind::Unreachable, e.to_string());
        }
    };

    let scheme = req
        .auth_scheme
        .as_deref()
        .unwrap_or(if shape.contains("anthropic") {
            "x_api_key"
        } else if req.api_key.as_ref().is_some_and(|k| !k.is_empty()) {
            "bearer"
        } else {
            "none"
        })
        .to_ascii_lowercase();

    let api_key = req.api_key.clone();

    let list_url = if shape == "ollama" {
        format!("{base}{}", local_health_path("ollama"))
    } else if shape.contains("anthropic") {
        format!("{base}/v1/models")
    } else {
        format!("{base}/v1/models")
    };

    let list_res = apply_auth(client.get(&list_url), &scheme, api_key.as_deref())
        .send()
        .await;
    let (list_ok, models, list_kind, list_msg) = match list_res {
        Ok(resp) if resp.status() == reqwest::StatusCode::UNAUTHORIZED => (
            false,
            vec![],
            ProbeErrorKind::Unauthorized,
            "401 listing models".into(),
        ),
        Ok(resp) if resp.status().is_success() => {
            let body = resp.json::<serde_json::Value>().await.unwrap_or_default();
            let models = extract_model_ids(&body);
            (true, models, ProbeErrorKind::Ok, "listed models".into())
        }
        Ok(resp) => (
            false,
            vec![],
            ProbeErrorKind::ShapeMismatch,
            format!("list returned {}", resp.status()),
        ),
        Err(e) => (
            false,
            vec![],
            ProbeErrorKind::Unreachable,
            format!("list failed: {e}"),
        ),
    };

    if !list_ok
        && matches!(
            list_kind,
            ProbeErrorKind::Unreachable | ProbeErrorKind::Unauthorized
        )
    {
        return ProbeResponse {
            ok: false,
            kind: list_kind,
            latency_ms: start.elapsed().as_millis() as u64,
            message: list_msg,
            models,
            embedding_dimension: None,
            chat_ok: false,
            embed_ok: false,
            list_ok: false,
        };
    }

    let model = req
        .model
        .clone()
        .unwrap_or_else(|| models.first().cloned().unwrap_or_else(|| "default".into()));

    let chat_ok = ping_chat(&client, &scheme, api_key.as_deref(), &shape, &base, &model).await;
    let (embed_ok, dim) = ping_embed(
        &client,
        &scheme,
        api_key.as_deref(),
        &shape,
        &base,
        req.embedding_model.as_deref(),
    )
    .await;

    let mut kind = ProbeErrorKind::Ok;
    let mut message = "provider reachable".to_string();
    if let Some(expected) = req.expected_dimension {
        if let Some(got) = dim {
            if got != expected {
                kind = ProbeErrorKind::DimMismatch;
                message = format!("embedding dimension {got} != expected {expected}");
            }
        }
    }
    if !chat_ok && kind == ProbeErrorKind::Ok {
        kind = ProbeErrorKind::ShapeMismatch;
        message = "chat ping failed".into();
    }
    if req
        .model
        .as_ref()
        .is_some_and(|m| !models.is_empty() && !models.iter().any(|x| x == m))
        && list_ok
        && kind == ProbeErrorKind::Ok
    {
        kind = ProbeErrorKind::ModelNotFound;
        message = format!(
            "model {} not in /v1/models",
            req.model.as_deref().unwrap_or("")
        );
    }

    let ok = kind == ProbeErrorKind::Ok && list_ok && chat_ok;
    ProbeResponse {
        ok,
        kind,
        latency_ms: start.elapsed().as_millis() as u64,
        message,
        models,
        embedding_dimension: dim,
        chat_ok,
        embed_ok,
        list_ok,
    }
}

fn fail(start: Instant, kind: ProbeErrorKind, message: String) -> ProbeResponse {
    ProbeResponse {
        ok: false,
        kind,
        latency_ms: start.elapsed().as_millis() as u64,
        message,
        models: vec![],
        embedding_dimension: None,
        chat_ok: false,
        embed_ok: false,
        list_ok: false,
    }
}

fn extract_model_ids(body: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = body.get("data").and_then(|d| d.as_array()) {
        return arr
            .iter()
            .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(|s| s.to_string()))
            .collect();
    }
    if let Some(arr) = body.get("models").and_then(|d| d.as_array()) {
        return arr
            .iter()
            .filter_map(|m| {
                m.get("name")
                    .or_else(|| m.get("model"))
                    .and_then(|i| i.as_str())
                    .map(|s| s.to_string())
            })
            .collect();
    }
    vec![]
}

fn apply_auth(
    b: reqwest::RequestBuilder,
    scheme: &str,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    match (scheme, api_key) {
        ("bearer", Some(k)) if !k.is_empty() => b.header("authorization", format!("Bearer {k}")),
        ("x_api_key" | "x-api-key", Some(k)) if !k.is_empty() => b.header("x-api-key", k),
        (_, Some(k)) if !k.is_empty() => b
            .header("authorization", format!("Bearer {k}"))
            .header("x-api-key", k),
        _ => b,
    }
}

async fn ping_chat(
    client: &reqwest::Client,
    scheme: &str,
    api_key: Option<&str>,
    shape: &str,
    base: &str,
    model: &str,
) -> bool {
    if shape == "ollama" {
        let r = apply_auth(client.post(format!("{base}/api/chat")), scheme, api_key)
            .json(&serde_json::json!({
                "model": model,
                "stream": false,
                "messages": [{"role":"user","content":"ping"}]
            }))
            .send()
            .await;
        return r.map(|x| x.status().is_success()).unwrap_or(false);
    }
    if shape.contains("anthropic") {
        let r = apply_auth(client.post(format!("{base}/v1/messages")), scheme, api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 8,
                "messages": [{"role":"user","content":"ping"}]
            }))
            .send()
            .await;
        return r.map(|x| x.status().is_success()).unwrap_or(false);
    }
    let r = apply_auth(
        client.post(format!("{base}/v1/chat/completions")),
        scheme,
        api_key,
    )
    .json(&serde_json::json!({
        "model": model,
        "max_tokens": 8,
        "messages": [{"role":"user","content":"ping"}]
    }))
    .send()
    .await;
    r.map(|x| x.status().is_success()).unwrap_or(false)
}

async fn ping_embed(
    client: &reqwest::Client,
    scheme: &str,
    api_key: Option<&str>,
    shape: &str,
    base: &str,
    model: Option<&str>,
) -> (bool, Option<usize>) {
    if shape.contains("anthropic") || shape == "ollama" {
        return (false, None);
    }
    let body = serde_json::json!({
        "model": model.unwrap_or("fake-embed"),
        "input": "dimension-probe"
    });
    let r = apply_auth(
        client.post(format!("{base}/v1/embeddings")),
        scheme,
        api_key,
    )
    .json(&body)
    .send()
    .await;
    match r {
        Ok(resp) if resp.status().is_success() => {
            let v = resp.json::<serde_json::Value>().await.unwrap_or_default();
            let dim = v
                .pointer("/data/0/embedding")
                .and_then(|e| e.as_array())
                .map(|a| a.len());
            (dim.is_some(), dim)
        }
        _ => (false, None),
    }
}

/// Live probe used by `/health` for the process default provider.
pub async fn probe_named_provider_reachable(name: &str) -> bool {
    let shape = name.to_ascii_lowercase();
    if !crate::locality::is_slow_local_provider(&shape) {
        return crate::providers::credentials::llm_provider_credentials_configured_by_name(name);
    }
    let base = match shape.as_str() {
        "ollama" => std::env::var("OLLAMA_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:11434".into()),
        "lmstudio" | "lm-studio" | "lm_studio" => std::env::var("LMSTUDIO_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:1234".into()),
        "omlx" => std::env::var("OMLX_HOST")
            .or_else(|_| std::env::var("OMLX_BASE_URL"))
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:9050".into()),
        "mlx-lm" | "mlx_lm" => std::env::var("MLX_LM_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:8083".into()),
        "llamacpp" | "llama-server" => std::env::var("LLAMACPP_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:8081".into()),
        "vllm-mlx" | "vllm_mlx" => std::env::var("VLLM_MLX_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:8082".into()),
        "mtplx" => std::env::var("MTPLX_HOST")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:9060".into()),
        _ => return false,
    };
    let req = ProbeRequest {
        shape,
        base_url: Some(base),
        model: None,
        embedding_model: None,
        api_key: None,
        auth_scheme: Some("none".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    };
    let r = probe_provider(req).await;
    r.list_ok || r.ok
}
