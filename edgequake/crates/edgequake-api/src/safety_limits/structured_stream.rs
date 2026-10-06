//! Hold the local inference permit and enforce the deadline through stream EOF.
use edgequake_llm::{traits::StreamChunk, LlmError, Result};
use futures::{stream::BoxStream, StreamExt};

pub(super) fn with_deadline(
    raw: BoxStream<'static, Result<StreamChunk>>,
    deadline: tokio::time::Instant,
    permit: Option<crate::local_inference_gate::LocalInferencePermit>,
) -> BoxStream<'static, Result<StreamChunk>> {
    futures::stream::unfold((Some(raw), permit), move |(raw, permit)| async move {
        let mut raw = raw?;
        match tokio::time::timeout_at(deadline, raw.next()).await {
            Ok(Some(Ok(chunk))) => Some((Ok(chunk), (Some(raw), permit))),
            Ok(Some(Err(error))) => Some((Err(error), (None, None))),
            Ok(None) => None,
            Err(_) => Some((Err(LlmError::Timeout), (None, None))),
        }
    })
    .boxed()
}

#[cfg(test)]
#[path = "structured_stream_tests.rs"]
mod tests;
