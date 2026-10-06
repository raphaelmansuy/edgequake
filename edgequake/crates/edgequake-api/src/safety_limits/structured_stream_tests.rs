use super::*;
use crate::safety_limits::{SafetyLimitedProviderWrapper, SafetyLimitsConfig};
use async_trait::async_trait;
use edgequake_llm::{
    ChatMessage, CompletionOptions, LLMProvider, LLMResponse, ToolChoice, ToolDefinition,
};
use std::{sync::Arc, time::Duration};

struct RecordingProvider {
    stall_start: bool,
}

#[async_trait]
impl LLMProvider for RecordingProvider {
    fn name(&self) -> &str {
        "mock"
    }
    fn model(&self) -> &str {
        "stream-test"
    }
    fn max_context_length(&self) -> usize {
        8192
    }
    async fn complete(&self, _: &str) -> Result<LLMResponse> {
        unreachable!()
    }
    async fn chat(&self, _: &[ChatMessage], _: Option<&CompletionOptions>) -> Result<LLMResponse> {
        unreachable!()
    }
    async fn complete_with_options(&self, _: &str, _: &CompletionOptions) -> Result<LLMResponse> {
        unreachable!()
    }
    fn supports_tool_streaming(&self) -> bool {
        true
    }
    async fn chat_with_tools_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        choice: Option<ToolChoice>,
        options: Option<&CompletionOptions>,
    ) -> Result<BoxStream<'static, Result<StreamChunk>>> {
        assert_eq!(messages[0].content, "question");
        assert!(tools.is_empty());
        assert!(matches!(choice, Some(ToolChoice::Auto(value)) if value == "none"));
        let options = options.unwrap();
        assert_eq!(options.max_tokens, Some(7));
        assert_eq!(options.reasoning_effort.as_deref(), Some("low"));
        assert_eq!(options.prompt_cache_key.as_deref(), Some("stable-key"));
        if self.stall_start {
            return futures::future::pending().await;
        }
        Ok(
            futures::stream::iter(vec![Ok(StreamChunk::Content("first".into()))])
                .chain(futures::stream::pending())
                .boxed(),
        )
    }
}

#[tokio::test]
async fn delegates_options_and_enforces_timeout_after_first_token() {
    let config = SafetyLimitsConfig {
        max_tokens: 7,
        timeout: Duration::from_millis(30),
        ..Default::default()
    };
    let wrapper = SafetyLimitedProviderWrapper::new(
        Arc::new(RecordingProvider { stall_start: false }),
        config,
    );
    assert!(wrapper.supports_tool_streaming());
    let options = CompletionOptions {
        max_tokens: Some(99),
        reasoning_effort: Some("low".into()),
        prompt_cache_key: Some("stable-key".into()),
        ..Default::default()
    };
    let mut stream = wrapper
        .chat_with_tools_stream(
            &[ChatMessage::user("question")],
            &[],
            Some(ToolChoice::none()),
            Some(&options),
        )
        .await
        .unwrap();
    assert!(
        matches!(stream.next().await.unwrap().unwrap(), StreamChunk::Content(text) if text == "first")
    );
    assert!(matches!(
        stream.next().await.unwrap(),
        Err(LlmError::Timeout)
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn startup_timeout_is_enforced() {
    let config = SafetyLimitsConfig {
        max_tokens: 7,
        timeout: Duration::from_millis(10),
        ..Default::default()
    };
    let wrapper = SafetyLimitedProviderWrapper::new(
        Arc::new(RecordingProvider { stall_start: true }),
        config,
    );
    let options = CompletionOptions {
        reasoning_effort: Some("low".into()),
        prompt_cache_key: Some("stable-key".into()),
        ..Default::default()
    };
    assert!(matches!(
        wrapper
            .chat_with_tools_stream(
                &[ChatMessage::user("question")],
                &[],
                Some(ToolChoice::none()),
                Some(&options)
            )
            .await,
        Err(LlmError::Timeout)
    ));
}

#[tokio::test]
async fn forwards_success_and_stops_after_provider_error() {
    let raw = futures::stream::iter(vec![
        Ok(StreamChunk::Content("text".into())),
        Err(LlmError::Timeout),
        Ok(StreamChunk::Content("must not appear".into())),
    ])
    .boxed();
    let mut stream = with_deadline(
        raw,
        tokio::time::Instant::now() + Duration::from_secs(1),
        None,
    );
    assert!(
        matches!(stream.next().await.unwrap().unwrap(), StreamChunk::Content(text) if text == "text")
    );
    assert!(stream.next().await.unwrap().is_err());
    assert!(stream.next().await.is_none());
    let raw = futures::stream::iter(vec![Ok(StreamChunk::Finished {
        reason: "stop".into(),
        ttft_ms: None,
        usage: None,
    })])
    .boxed();
    let mut stream = with_deadline(
        raw,
        tokio::time::Instant::now() + Duration::from_secs(1),
        None,
    );
    assert!(matches!(
        stream.next().await.unwrap().unwrap(),
        StreamChunk::Finished { .. }
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn releases_permit_on_completion_error_and_cancellation() {
    for outcome in ["complete", "error", "cancel"] {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let raw: BoxStream<'static, Result<StreamChunk>> = match outcome {
            "complete" => futures::stream::empty().boxed(),
            "error" => futures::stream::iter(vec![Err(LlmError::Timeout)]).boxed(),
            _ => futures::stream::pending().boxed(),
        };
        let mut stream = with_deadline(
            raw,
            tokio::time::Instant::now() + Duration::from_secs(1),
            Some(crate::local_inference_gate::LocalInferencePermit::Semaphore(permit)),
        );
        assert_eq!(semaphore.available_permits(), 0);
        if outcome != "cancel" {
            let _ = stream.next().await;
        }
        drop(stream);
        assert_eq!(semaphore.available_permits(), 1, "{outcome}");
    }
}
