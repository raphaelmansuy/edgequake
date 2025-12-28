# Gap Analysis: EdgeQuake vs Open-WebUI Streaming Token Handling

## Executive Summary

This document provides a comprehensive gap analysis comparing EdgeQuake's current streaming chat implementation with open-webui's production-grade implementation. The analysis identifies critical gaps in database write optimization, caching, and message storage patterns.

## 1. Architecture Comparison

### 1.1 EdgeQuake Current Architecture

**Location**: `edgequake/crates/edgequake-api/src/handlers/chat.rs`

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│   SSE API   │────▶│  Query      │
│  (Browser)  │     │  /stream    │     │  Engine     │
└─────────────┘     └─────────────┘     └─────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │  PostgreSQL │
                    │   (Direct)  │
                    └─────────────┘
```

**Current Flow** (lines 370-678 in chat.rs):

1. Validate request
2. Create/get conversation BEFORE streaming
3. Save user message BEFORE streaming
4. Start streaming - accumulate `full_content` in memory
5. After streaming completes, save assistant message with `full_content`
6. Send Done event

### 1.2 Open-WebUI Architecture

**Location**: `open-webui/backend/open_webui/socket/main.py`

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client    │────▶│  Socket.IO  │────▶│   Event     │────▶│   LLM API   │
│  (Browser)  │     │  WebSocket  │     │   Emitter   │     │   (OpenAI)  │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                   │
                           │                   ▼
                           │            ┌─────────────┐
                           │            │   Redis     │
                           │            │   (Cache)   │
                           │            └─────────────┘
                           │                   │
                           ▼                   ▼
                    ┌─────────────┐     ┌─────────────┐
                    │  PostgreSQL │◀────│   Debounce  │
                    │             │     │   Tasks     │
                    └─────────────┘     └─────────────┘
```

**Key Patterns** (lines 683-754 in socket/main.py):

1. Event emitter pattern for real-time updates
2. Debounced database saves (0.5s delay via `create_task`)
3. Redis-backed session/usage pools
4. Incremental upsert pattern for message content
5. Task cancellation on new updates

## 2. Critical Gaps Identified

### 2.1 GAP: No Debouncing for Database Writes

**EdgeQuake Current**:

- Single atomic write AFTER streaming completes
- Full content accumulated in memory during stream
- Risk: Memory pressure with very long responses

**Open-WebUI Approach**:

```python
# socket/main.py line 616-619
async def debounced_save():
    await asyncio.sleep(0.5)
    await document_save_handler(document_id, data.get("data", {}), SESSION_POOL.get(sid))

if data.get("data"):
    await create_task(REDIS, debounced_save(), document_id)
```

**Impact**: HIGH

- EdgeQuake approach is actually BETTER for data integrity (atomic final write)
- BUT lacks intermediate checkpoint saves for crash recovery
- No streaming progress persistence

### 2.2 GAP: No LRU Cache for Conversations/Messages

**EdgeQuake Current**:

- Every conversation/message fetch goes directly to PostgreSQL
- No caching layer for recently accessed data
- Hot conversations cause repeated DB queries

**Open-WebUI Approach**:

```python
# socket/main.py lines 107-130
if WEBSOCKET_MANAGER == "redis":
    MODELS = RedisDict(f"{REDIS_KEY_PREFIX}:models", ...)
    SESSION_POOL = RedisDict(f"{REDIS_KEY_PREFIX}:session_pool", ...)
    USAGE_POOL = RedisDict(f"{REDIS_KEY_PREFIX}:usage_pool", ...)
```

**Impact**: MEDIUM-HIGH

- Active conversations cause repeated queries
- No read-through caching
- Missing invalidation strategy

### 2.3 GAP: Token Counting vs Full Response Storage

**EdgeQuake Current** (lines 602-613):

```rust
while let Some(chunk_result) = stream.next().await {
    match chunk_result {
        Ok(text) => {
            full_content.push_str(&text);
            tokens_used += 1;  // ❌ This counts CHUNKS, not tokens!
            // ...
        }
    }
}
```

**Issues**:

1. `tokens_used` increments per CHUNK, not per token
2. No actual token counting from API response
3. Missing `usage` data from LLM API response

**Open-WebUI Approach**:

- Relies on upstream API's token counts
- Stores actual `usage` object from OpenAI response

**Impact**: HIGH for billing/metrics accuracy

### 2.4 GAP: No Streaming Progress Checkpoints

**EdgeQuake Current**:

- All-or-nothing storage: complete response or nothing
- Client disconnect = lost partial response
- No recovery mechanism

**Open-WebUI Approach** (lines 715-754):

```python
if event_data["type"] == "message":
    message = Chats.get_message_by_id_and_message_id(
        request_info["chat_id"],
        request_info["message_id"],
    )
    if message:
        content = message.get("content", "")
        content += event_data.get("data", {}).get("content", "")
        Chats.upsert_message_to_chat_by_id_and_message_id(
            request_info["chat_id"],
            request_info["message_id"],
            {"content": content},
        )
```

**Impact**: MEDIUM

- Partial response recovery
- Resume capability after disconnection

### 2.5 GAP: No Task Cancellation on Update

**EdgeQuake Current**:

- No background task management
- No cancellation of pending operations

**Open-WebUI Approach** (tasks.py lines 149-166):

```python
async def stop_item_tasks(redis: Redis, item_id: str):
    """Stop all tasks associated with a specific item ID."""
    task_ids = await list_task_ids_by_item_id(redis, item_id)
    for task_id in task_ids:
        result = await stop_task(redis, task_id)
```

**Impact**: LOW-MEDIUM

- Resource cleanup
- Prevent stale updates

## 3. Edge Cases Analysis

### 3.1 Client Disconnection During Stream

| Scenario        | EdgeQuake         | Open-WebUI                 |
| --------------- | ----------------- | -------------------------- |
| Full disconnect | Response lost     | Partial saved via debounce |
| Reconnect       | Start new request | Resume not supported       |
| Recovery        | None              | Check last saved content   |

### 3.2 Very Long Responses (>100KB)

| Scenario     | EdgeQuake             | Open-WebUI            |
| ------------ | --------------------- | --------------------- |
| Memory usage | Accumulates in String | Incremental saves     |
| Timeout risk | Single final write    | Multiple small writes |
| DB pressure  | One large INSERT      | Many small UPDATEs    |

### 3.3 Concurrent Requests Same Conversation

| Scenario            | EdgeQuake       | Open-WebUI        |
| ------------------- | --------------- | ----------------- |
| Race condition      | Possible        | Task cancellation |
| Message ordering    | By timestamp    | Parent ID chain   |
| Conflict resolution | Last write wins | Merge pattern     |

## 4. Recommendations Priority Matrix

| Gap                        | Priority | Effort | Impact |
| -------------------------- | -------- | ------ | ------ |
| Token counting accuracy    | P0       | Low    | High   |
| LRU cache for hot data     | P1       | Medium | High   |
| Debounced checkpoint saves | P2       | Medium | Medium |
| Full API response storage  | P0       | Low    | High   |
| Task management            | P3       | High   | Low    |

## 5. Key Insights from Open-WebUI

### 5.1 Message Storage Pattern (models/chats.py)

```python
def upsert_message_to_chat_by_id_and_message_id(
    self, id: str, message_id: str, message: dict
) -> Optional[ChatModel]:
    # Atomic upsert with merge semantics
    history["messages"][message_id] = {
        **history["messages"][message_id],
        **message,  # Merge new fields
    }
```

### 5.2 Event-Driven Updates (socket/main.py)

```python
async def __event_emitter__(event_data):
    await sio.emit("events", {...}, room=f"user:{user_id}")
    if update_db and message_id:
        if event_data["type"] == "replace":
            # Full replacement (final message)
            content = event_data.get("data", {}).get("content", "")
            Chats.upsert_message_to_chat_by_id_and_message_id(...)
```

### 5.3 Debounce Implementation (tasks.py)

```python
async def create_task(redis, coroutine, id=None):
    task_id = str(uuid4())
    task = asyncio.create_task(coroutine)
    task.add_done_callback(lambda t: asyncio.create_task(cleanup_task(redis, task_id, id)))
    tasks[task_id] = task
    # ...
```

## 6. Next Steps

1. **Document 02**: Debounce Strategy Design
2. **Document 03**: LRU Cache Architecture
3. **Document 04**: Message Storage Strategy
4. **Document 05**: Unit Test Specifications
5. **Document 06**: E2E Test Specifications
6. **Document 07**: Implementation Plan

---

_Analysis Date: 2024-12-28_
_EdgeQuake Version: Current main branch_
_Open-WebUI Version: As referenced in workspace_
