# OODA 16 — Decide: Python SDK Fix

1. Fix `types/chat.py` → EdgeQuake-native format
2. Fix `resources/chat.py` → `complete(message: str)`
3. Fix `types/conversations.py` → add `mode`, make `message_count` optional
4. Fix E2E tests → use correct method signatures
5. Run E2E against live backend → verify 29/29 pass
