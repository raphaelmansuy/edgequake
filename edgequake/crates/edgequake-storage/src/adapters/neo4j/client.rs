//! Single-node Neo4j Query API v2 transport.

use std::fmt;
use std::time::Duration;

use edgequake_storage_contracts::{AccessError, AccessResult};
use reqwest::header::{HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::{Method, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::errors::{classify_http, classify_query};

const DEFAULT_DATABASE: &str = "neo4j";
const DEFAULT_TIMEOUT_SECONDS: u64 = 10;
const AFFINITY_HEADER: &str = "neo4j-cluster-affinity";

#[derive(Clone)]
pub struct Neo4jConfig {
    pub base_url: String,
    pub database: String,
    pub username: String,
    password: String,
    pub timeout: Duration,
}

impl fmt::Debug for Neo4jConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Neo4jConfig")
            .field("base_url", &self.base_url)
            .field("database", &self.database)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Neo4jConfig {
    pub fn from_env() -> AccessResult<Self> {
        let topology =
            std::env::var("EDGEQUAKE_NEO4J_TOPOLOGY").unwrap_or_else(|_| "single".into());
        if !matches!(topology.trim(), "single" | "single_node" | "single-node") {
            return Err(AccessError::UnsupportedCapability(format!(
                "Neo4j Query API explicit transactions require single-node topology; got '{topology}'"
            )));
        }
        Self::new(
            required_env("EDGEQUAKE_GRAPH_URL")?,
            std::env::var("EDGEQUAKE_NEO4J_DATABASE").unwrap_or_else(|_| DEFAULT_DATABASE.into()),
            required_env("EDGEQUAKE_NEO4J_USER")?,
            required_env("EDGEQUAKE_NEO4J_PASSWORD")?,
        )
    }

    pub fn new(
        base_url: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> AccessResult<Self> {
        let base_url = base_url.into();
        let parsed = Url::parse(base_url.trim()).map_err(|error| {
            AccessError::InvalidInput(format!("invalid EDGEQUAKE_GRAPH_URL: {error}"))
        })?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err(AccessError::InvalidInput(
                "EDGEQUAKE_GRAPH_URL must be an absolute http(s) URL".into(),
            ));
        }
        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(AccessError::InvalidInput(
                "EDGEQUAKE_GRAPH_URL must not contain a query or fragment".into(),
            ));
        }
        let database = database.into();
        validate_path_segment(&database, "Neo4j database")?;
        let username = username.into();
        let password = password.into();
        if username.is_empty() || password.is_empty() {
            return Err(AccessError::InvalidInput(
                "Neo4j username and password must not be empty".into(),
            ));
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            database,
            username,
            password,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
        })
    }

    fn transaction_url(&self) -> String {
        format!("{}/db/{}/query/v2/tx", self.base_url, self.database)
    }
}

#[derive(Clone)]
pub struct Neo4jClient {
    http: reqwest::Client,
    config: Neo4jConfig,
}

impl Neo4jClient {
    pub fn new(config: Neo4jConfig) -> AccessResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| AccessError::Unavailable(error.to_string()))?;
        Ok(Self { http, config })
    }

    pub fn config(&self) -> &Neo4jConfig {
        &self.config
    }

    pub async fn verify_connectivity(&self) -> AccessResult<()> {
        let mut transaction = self.open_transaction().await?;
        match transaction
            .execute("RETURN 1 AS edgequake_ready", serde_json::json!({}))
            .await
        {
            Ok(_) => {
                transaction.rollback().await?;
                Ok(())
            }
            Err(error) => {
                transaction.rollback().await?;
                Err(error)
            }
        }
    }

    pub(crate) async fn open_transaction(&self) -> AccessResult<Neo4jTransaction> {
        let request = self
            .authenticated(Method::POST, &self.config.transaction_url())
            .json(&serde_json::json!({}));
        let raw = self.send(request, false).await?;
        let transaction = raw.response.transaction.ok_or_else(|| {
            AccessError::CorruptData("Neo4j transaction response omitted transaction.id".into())
        })?;
        validate_path_segment(&transaction.id, "Neo4j transaction ID")?;
        Ok(Neo4jTransaction {
            client: self.clone(),
            id: transaction.id,
            affinity: raw.affinity,
            closed: false,
        })
    }

    fn authenticated(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
    }

    async fn send(
        &self,
        request: reqwest::RequestBuilder,
        commit_may_be_ambiguous: bool,
    ) -> AccessResult<RawResponse> {
        let response = request.send().await.map_err(|error| {
            if commit_may_be_ambiguous {
                AccessError::UnknownOutcome(format!(
                    "Neo4j commit response was not observed: {error}"
                ))
            } else if error.is_timeout() {
                AccessError::DeadlineExceeded(error.to_string())
            } else {
                AccessError::Unavailable(error.to_string())
            }
        })?;
        let status = response.status();
        let affinity = response.headers().get(AFFINITY_HEADER).cloned();
        let body = response.text().await.map_err(|error| {
            if commit_may_be_ambiguous {
                AccessError::UnknownOutcome(format!("Neo4j commit body was not observed: {error}"))
            } else {
                AccessError::Unavailable(error.to_string())
            }
        })?;
        let response = parse_response(status, &body)?;
        Ok(RawResponse { response, affinity })
    }
}

pub(crate) struct Neo4jTransaction {
    client: Neo4jClient,
    id: String,
    affinity: Option<HeaderValue>,
    closed: bool,
}

impl Neo4jTransaction {
    pub(crate) async fn execute(
        &mut self,
        statement: &str,
        parameters: Value,
    ) -> AccessResult<QueryResponse> {
        if self.closed {
            return Err(AccessError::InvalidInput(
                "Neo4j transaction is already closed".into(),
            ));
        }
        let request = self
            .with_affinity(self.client.authenticated(Method::POST, &self.url()))
            .json(&QueryRequest {
                statement,
                parameters,
            });
        let raw = self.client.send(request, false).await?;
        if let Some(transaction) = &raw.response.transaction {
            if transaction.id != self.id {
                return Err(AccessError::CorruptData(
                    "Neo4j changed transaction identity mid-transaction".into(),
                ));
            }
        }
        Ok(raw.response)
    }

    pub(crate) async fn commit(mut self) -> AccessResult<Vec<String>> {
        let url = format!("{}/commit", self.url());
        let request = self.with_affinity(
            self.client
                .authenticated(Method::POST, &url)
                .json(&serde_json::json!({})),
        );
        let response = self.client.send(request, true).await?;
        self.closed = true;
        Ok(response.response.bookmarks)
    }

    pub(crate) async fn rollback(mut self) -> AccessResult<()> {
        let request = self.with_affinity(self.client.authenticated(Method::DELETE, &self.url()));
        self.client.send(request, false).await?;
        self.closed = true;
        Ok(())
    }

    fn url(&self) -> String {
        format!("{}/{}", self.client.config.transaction_url(), self.id)
    }

    fn with_affinity(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.affinity {
            Some(value) => request.header(AFFINITY_HEADER, value),
            None => request,
        }
    }
}

#[derive(Debug, Serialize)]
struct QueryRequest<'a> {
    statement: &'a str,
    parameters: Value,
}

#[derive(Debug)]
struct RawResponse {
    response: QueryResponse,
    affinity: Option<HeaderValue>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryResponse {
    #[serde(default)]
    pub data: QueryData,
    #[serde(default)]
    pub bookmarks: Vec<String>,
    #[serde(default)]
    pub errors: Vec<Neo4jQueryError>,
    pub transaction: Option<TransactionInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct QueryData {
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub values: Vec<Vec<Value>>,
}

impl QueryData {
    pub fn object_rows(&self) -> AccessResult<Vec<Map<String, Value>>> {
        self.values
            .iter()
            .map(|values| {
                if values.len() != self.fields.len() {
                    return Err(AccessError::CorruptData(
                        "Neo4j result field/value cardinality mismatch".into(),
                    ));
                }
                Ok(self
                    .fields
                    .iter()
                    .cloned()
                    .zip(values.iter().cloned())
                    .collect())
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct TransactionInfo {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Neo4jQueryError {
    pub code: String,
    pub message: String,
}

pub(crate) fn parse_response(status: StatusCode, body: &str) -> AccessResult<QueryResponse> {
    if !status.is_success() {
        return Err(classify_http(status, body));
    }
    let response = if body.trim().is_empty() {
        QueryResponse::default()
    } else {
        serde_json::from_str::<QueryResponse>(body).map_err(|error| {
            AccessError::CorruptData(format!("invalid Neo4j Query API response: {error}"))
        })?
    };
    if !response.errors.is_empty() {
        return Err(classify_query(&response.errors));
    }
    Ok(response)
}

fn required_env(name: &str) -> AccessResult<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AccessError::InvalidInput(format!("{name} is required for Neo4j")))
}

fn validate_path_segment(value: &str, field: &str) -> AccessResult<()> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(AccessError::InvalidInput(format!(
            "{field} contains unsafe URL path characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_http_body_errors_are_failures() {
        let body = r#"{
          "errors": [{
            "code": "Neo.ClientError.Statement.SyntaxError",
            "message": "invalid input"
          }]
        }"#;
        let error = parse_response(StatusCode::ACCEPTED, body).unwrap_err();
        assert!(matches!(error, AccessError::InvalidInput(_)));
        assert!(error.to_string().contains("SyntaxError"));
    }

    #[test]
    fn accepted_empty_errors_are_success() {
        let response = parse_response(
            StatusCode::ACCEPTED,
            r#"{"data":{"fields":[],"values":[]},"errors":[]}"#,
        )
        .unwrap();
        assert!(response.errors.is_empty());
    }

    #[test]
    fn config_rejects_path_injection() {
        let error = Neo4jConfig::new("http://localhost:7474", "../system", "neo4j", "password")
            .unwrap_err();
        assert!(matches!(error, AccessError::InvalidInput(_)));
    }
}
