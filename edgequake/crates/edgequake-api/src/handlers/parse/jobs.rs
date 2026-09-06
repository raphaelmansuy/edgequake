//! In-memory TTL job store for async parse (SPEC-094).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use uuid::Uuid;

use super::options::ResolvedParseOptions;
use super::service::run_parse;
use super::types::{ParseAsyncAccepted, ParseJobErrorBody, ParseJobStatusResponse, ParseResponse};
use crate::error::ApiResult;

const DEFAULT_TTL: Duration = Duration::from_secs(3600);
const DEFAULT_MAX_CONCURRENT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

struct JobRecord {
    status: JobStatus,
    created_at: Instant,
    request_id: String,
    /// SPEC-146: minting principal; unauthorized GET → 404.
    owner_principal: Option<String>,
    result: Option<ParseResponse>,
    error: Option<ParseJobErrorBody>,
    /// Kept until worker starts, then dropped.
    pdf_bytes: Option<Vec<u8>>,
    resolved: Option<ResolvedParseOptions>,
}

/// Process-wide admission + TTL store for async parse jobs.
#[derive(Clone)]
pub struct ParseJobStore {
    inner: Arc<RwLock<HashMap<String, JobRecord>>>,
    semaphore: Arc<Semaphore>,
    ttl: Duration,
    max_concurrent: usize,
}

impl Default for ParseJobStore {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_CONCURRENT, DEFAULT_TTL)
    }
}

impl ParseJobStore {
    pub fn new(max_concurrent: usize, ttl: Duration) -> Self {
        let max_concurrent = max_concurrent.clamp(1, 16);
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            ttl,
            max_concurrent,
        }
    }

    pub fn from_env() -> Self {
        let max = std::env::var("EDGEQUAKE_PARSE_MAX_CONCURRENT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MAX_CONCURRENT);
        Self::new(max, DEFAULT_TTL)
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    pub async fn acquire_permit(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().acquire_owned().await.ok()
    }

    /// Enqueue a job and spawn a background worker.
    pub async fn enqueue(
        &self,
        pdf_bytes: Vec<u8>,
        resolved: ResolvedParseOptions,
        request_id: String,
        owner_principal: Option<String>,
    ) -> ApiResult<ParseAsyncAccepted> {
        self.purge_expired().await;
        let job_id = format!("pr_{}", Uuid::new_v4().simple());
        {
            let mut map = self.inner.write().await;
            map.insert(
                job_id.clone(),
                JobRecord {
                    status: JobStatus::Pending,
                    created_at: Instant::now(),
                    request_id: request_id.clone(),
                    owner_principal,
                    result: None,
                    error: None,
                    pdf_bytes: Some(pdf_bytes),
                    resolved: Some(resolved),
                },
            );
        }

        let store = self.clone();
        let worker_id = job_id.clone();
        tokio::spawn(async move {
            store.run_job(worker_id).await;
        });

        Ok(ParseAsyncAccepted {
            job_id,
            status: JobStatus::Pending.as_str().to_string(),
            request_id,
        })
    }

    /// Insert a completed job without a worker (SPEC-146 parse IDOR gates).
    pub async fn seed_completed_job(&self, job_id: &str, owner_principal: Option<String>) {
        let mut map = self.inner.write().await;
        map.insert(
            job_id.to_string(),
            JobRecord {
                status: JobStatus::Completed,
                created_at: Instant::now(),
                request_id: "seed".into(),
                owner_principal,
                result: None,
                error: None,
                pdf_bytes: None,
                resolved: None,
            },
        );
    }

    async fn run_job(&self, job_id: String) {
        let _permit = match self.acquire_permit().await {
            Some(p) => p,
            None => {
                self.fail_job(
                    &job_id,
                    "parse.backend_unavailable",
                    "Parse admission semaphore closed",
                )
                .await;
                return;
            }
        };

        let (bytes, resolved, request_id) = {
            let mut map = self.inner.write().await;
            let Some(job) = map.get_mut(&job_id) else {
                return;
            };
            job.status = JobStatus::Running;
            let bytes = job.pdf_bytes.take().unwrap_or_default();
            let resolved = job.resolved.take();
            (bytes, resolved, job.request_id.clone())
        };

        let Some(resolved) = resolved else {
            self.fail_job(&job_id, "parse.invalid_request", "Missing job options")
                .await;
            return;
        };

        match run_parse(&bytes, &resolved, Some(request_id)).await {
            Ok(result) => {
                let mut map = self.inner.write().await;
                if let Some(job) = map.get_mut(&job_id) {
                    job.status = JobStatus::Completed;
                    job.result = Some(result);
                    job.pdf_bytes = None;
                    job.resolved = None;
                }
            }
            Err(err) => {
                self.fail_job(&job_id, err.code(), &err.to_string()).await;
            }
        }
    }

    async fn fail_job(&self, job_id: &str, code: &str, message: &str) {
        let mut map = self.inner.write().await;
        if let Some(job) = map.get_mut(job_id) {
            job.status = JobStatus::Failed;
            job.error = Some(ParseJobErrorBody {
                code: code.to_string(),
                message: message.to_string(),
            });
            job.pdf_bytes = None;
            job.resolved = None;
        }
    }

    pub async fn get(&self, job_id: &str) -> Option<ParseJobStatusResponse> {
        self.get_for(job_id, None).await
    }

    /// SPEC-146: when `caller_principal` is `Some`, require owner match (else 404).
    pub async fn get_for(
        &self,
        job_id: &str,
        caller_principal: Option<&str>,
    ) -> Option<ParseJobStatusResponse> {
        self.purge_expired().await;
        let map = self.inner.read().await;
        let job = map.get(job_id)?;
        // SPEC-146: when a caller principal is supplied (ABAC on), unbound jobs
        // are fail-closed — not world-readable.
        if let Some(caller) = caller_principal {
            match job.owner_principal.as_deref() {
                Some(owner) if owner == caller => {}
                Some(_) | None => return None,
            }
        }
        Some(ParseJobStatusResponse {
            job_id: job_id.to_string(),
            status: job.status.as_str().to_string(),
            result: job.result.clone(),
            error: job.error.clone(),
            request_id: job.request_id.clone(),
        })
    }

    async fn purge_expired(&self) {
        let mut map = self.inner.write().await;
        map.retain(|_, job| job.created_at.elapsed() < self.ttl);
    }
}

/// GET /api/v1/parse/jobs/{id}
#[utoipa::path(
    get,
    path = "/api/v1/parse/jobs/{id}",
    params(
        ("id" = String, Path, description = "Parse job id returned by POST /parse")
    ),
    responses(
        (status = 200, description = "Job status", body = ParseJobStatusResponse),
        (status = 404, description = "Unknown or expired job")
    ),
    tag = "Parse"
)]
pub async fn get_parse_job(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    tenant_ctx: crate::middleware::TenantContext,
) -> ApiResult<axum::Json<ParseJobStatusResponse>> {
    let caller = if state.security.doc_abac {
        tenant_ctx.user_id.as_deref().map(|u| {
            let p = edgequake_authz::PrincipalId::from_auth_user_id(u);
            format!("{}:{}", p.kind_str(), p.id_str())
        })
    } else {
        None
    };
    match state
        .parse_jobs
        .get_for(&id, caller.as_deref())
        .await
    {
        Some(status) => Ok(axum::Json(status)),
        None => Err(super::errors::ParseErrorCode::JobNotFound
            .into_api_error(format!("Parse job not found: {id}"))),
    }
}
