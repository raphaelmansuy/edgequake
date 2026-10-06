//! Structured provider events → answer tokens, with completion-only caching.

use super::super::TokenStream;
use crate::error::QueryError;
use edgequake_llm::traits::StreamChunk;
use futures::{stream::BoxStream, StreamExt};
use std::future::Future;

pub(super) fn completion_options(
    effort: Option<&str>,
    llm: &dyn edgequake_llm::traits::LLMProvider,
) -> edgequake_llm::traits::CompletionOptions {
    edgequake_llm::traits::CompletionOptions {
        reasoning_effort: effort
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        ..Default::default()
    }
    .with_provider_prompt_cache("query", llm.name(), llm.model())
}

pub(super) fn text_deltas(
    raw: BoxStream<'static, edgequake_llm::Result<StreamChunk>>,
) -> TokenStream {
    // Some providers emit Finished; OpenAI-compatible providers consume [DONE]
    // internally and expose successful EOF instead. Transport failures are Err.
    futures::stream::unfold(Some(raw), |raw| async move {
        let mut raw = raw?;
        while let Some(event) = raw.next().await {
            match event {
                Ok(StreamChunk::Content(text)) if !text.is_empty() => {
                    return Some((Ok(text), Some(raw)))
                }
                Ok(StreamChunk::Finished {
                    usage: Some(usage), ..
                }) => {
                    edgequake_observability::record_gen_ai_usage(
                        Some(usage.prompt_tokens as u64),
                        Some(usage.completion_tokens as u64),
                    );
                }
                Err(error) => return Some((Err(QueryError::from(error)), None)),
                // Transport, progress, reasoning and tool metadata are not answer text.
                _ => {}
            }
        }
        None
    })
    .boxed()
}

/// Poll one input chunk at a time. Dropping or failing the stream never caches
/// a partial answer; the callback runs only after successful nonempty EOF.
pub(super) fn on_complete<F, Fut>(stream: TokenStream, callback: F) -> TokenStream
where
    F: FnOnce(String) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    futures::stream::unfold(
        (stream, String::new(), false, Some(callback)),
        |(mut stream, mut answer, mut failed, mut callback)| async move {
            match stream.next().await {
                Some(chunk) => {
                    match &chunk {
                        Ok(text) => answer.push_str(text),
                        Err(_) => failed = true,
                    }
                    Some((chunk, (stream, answer, failed, callback)))
                }
                None => {
                    if !failed && !answer.is_empty() {
                        if let Some(callback) = callback.take() {
                            callback(answer).await;
                        }
                    }
                    None
                }
            }
        },
    )
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn forwards_content_and_errors_without_exposing_metadata() {
        let raw = futures::stream::iter(vec![
            Ok(StreamChunk::Connected {
                phase: "headers".into(),
            }),
            Ok(StreamChunk::ThinkingContent {
                text: "private reasoning".into(),
                tokens_used: None,
                budget_total: None,
            }),
            Ok(StreamChunk::Content("first ".into())),
            Ok(StreamChunk::Content("second".into())),
            Err(edgequake_llm::LlmError::NotSupported(
                "provider failure".into(),
            )),
        ])
        .boxed();
        let mut stream = text_deltas(raw);
        assert_eq!(stream.next().await.unwrap().unwrap(), "first ");
        assert_eq!(stream.next().await.unwrap().unwrap(), "second");
        assert!(stream.next().await.unwrap().is_err());
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn clean_provider_eof_does_not_require_optional_finished_event() {
        let raw = futures::stream::iter(vec![Ok(StreamChunk::Content("answer".into()))]).boxed();
        let mut stream = text_deltas(raw);
        assert_eq!(stream.next().await.unwrap().unwrap(), "answer");
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn caches_only_successful_complete_answers() {
        for outcome in ["complete", "error", "cancel", "empty"] {
            let saved = Arc::new(Mutex::new(None));
            let target = saved.clone();
            let chunks = match outcome {
                "error" => vec![
                    Ok("partial".into()),
                    Err(QueryError::Internal("failed".into())),
                ],
                "empty" => vec![],
                _ => vec![Ok("first ".into()), Ok("second".into())],
            };
            let mut stream = on_complete(
                futures::stream::iter(chunks).boxed(),
                move |answer| async move {
                    *target.lock().unwrap() = Some(answer);
                },
            );
            if outcome == "cancel" {
                assert_eq!(stream.next().await.unwrap().unwrap(), "first ");
            } else {
                while stream.next().await.is_some() {}
            }
            drop(stream);
            assert_eq!(
                saved.lock().unwrap().as_deref(),
                (outcome == "complete").then_some("first second"),
                "{outcome}"
            );
        }
    }
}
