# OODA 16 — Orient: Python SDK Fix

## Root Cause
SDK types were designed to imitate OpenAI chat API, but EdgeQuake has its own native format.

## Approach
1. Replace chat types: `message: str` instead of `messages: list[ChatMessage]`
2. Replace chat response: `conversation_id, content, sources, stats` instead of `choices[].message`
3. Fix `ConversationInfo.message_count` to accept `None`
4. Add `mode` field to `ConversationCreate`
5. Fix E2E tests to use correct method signatures
