# OODA 16 — Observe: Python SDK Fix

## Findings
- Python SDK chat types used OpenAI format (`messages: list[ChatMessage]`) — WRONG
- Backend uses `message` (singular string) + `conversation_id` response
- `ConversationInfo.message_count` typed as `int` but API returns `null` → Pydantic validation error
- `conversations.create()` takes keyword args, not `ConversationCreate` object
- `folders.create()` takes `name: str`, not `FolderCreate` object
- `ConversationCreate` was missing `mode` field that backend supports
