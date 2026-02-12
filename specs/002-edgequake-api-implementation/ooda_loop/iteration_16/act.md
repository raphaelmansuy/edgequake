# OODA 16 — Act: Python SDK Fix

## Changes Made
- `sdks/python/edgequake/types/chat.py` — Replaced OpenAI format with EdgeQuake native format
- `sdks/python/edgequake/resources/chat.py` — `complete(message: str)` + `stream(message: str)`
- `sdks/python/edgequake/types/conversations.py` — Added `mode` to `ConversationCreate`, made `message_count` optional
- `sdks/python/tests/test_e2e.py` — Added `tenant_client` fixture, fixed conversations/folders test calls

## E2E Results
**29/29 passed, 0 failed, 0 skipped** (18.53s)
