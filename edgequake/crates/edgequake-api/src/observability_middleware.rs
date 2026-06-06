//! Unified HTTP observability middleware (DRY: request ID + spans + logs + metrics).

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use edgequake_observability::{
    harvest_propagation_headers, record_http_request, record_http_status, resolve_request_id,
    scope_request_id, synthesize_traceparent_from_request_id, with_http_span, RequestContext,
    REQUEST_ID_HEADER, TRACEPARENT_HEADER, TRACESTATE_HEADER,
};
use std::time::Instant;
use tracing::debug;

/// Single middleware: correlation ID, per-request span, Prometheus metrics, response headers.
pub async fn observability_middleware(mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let headers = request.headers();

    let request_id = resolve_request_id(headers);
    let traceparent = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let tracestate = headers
        .get(TRACESTATE_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let propagation = harvest_propagation_headers(headers);
    let ctx = RequestContext::new(request_id.clone()).with_trace(traceparent.clone(), tracestate);
    request.extensions_mut().insert(ctx);
    request.extensions_mut().insert(propagation);

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        request.headers_mut().insert(REQUEST_ID_HEADER, value);
    }

    let start = Instant::now();
    let incoming_headers = request.headers().clone();

    let mut response = scope_request_id(request_id.clone(), async {
        with_http_span(
            &request_id,
            method.as_str(),
            &path,
            traceparent.as_deref(),
            Some(&incoming_headers),
            async {
                let response = next.run(request).await;
                let status = response.status().as_u16();
                record_http_status(status);
                response
            },
        )
        .await
    })
    .await;

    let duration = start.elapsed();
    let status = response.status().as_u16();

    record_http_request(method.as_str(), &path, status, duration.as_secs_f64());

    // 4xx/5xx body errors are logged in `ApiError::into_response` with explicit context.
    if status >= 400 {
        debug!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = status,
            duration_ms = duration.as_millis(),
            "HTTP response completed with error status"
        );
    } else {
        debug!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = status,
            duration_ms = duration.as_millis(),
            "HTTP request completed"
        );
    }

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }

    // Echo inbound traceparent or inject server span context for downstream clients
    if traceparent.is_none() {
        #[cfg(feature = "otel")]
        edgequake_observability::inject_current_context(response.headers_mut());
    }
    if response.headers().get(TRACEPARENT_HEADER).is_none() {
        let outbound_traceparent = traceparent
            .clone()
            .or_else(|| synthesize_traceparent_from_request_id(&request_id));
        if let Some(tp) = outbound_traceparent {
            if let Ok(value) = HeaderValue::from_str(&tp) {
                response.headers_mut().insert(TRACEPARENT_HEADER, value);
            }
        }
    }

    response
}
