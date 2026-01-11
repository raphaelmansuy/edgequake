# OODA Loop 51: Fix Provider Override Error Handling

**Date:** 2026-01-11 22:30 UTC  
**Status:** 🔄 In Progress  
**Focus:** Ensure explicit provider selection errors are returned to user, not silently ignored

---

## 🔍 OBSERVE

### Current Behavior

When user explicitly selects OpenAI provider in UI (via dropdown):

1. ✅ Backend has Ollama as default (health check confirms)
2. ✅ OpenAI API key validation exists in `create_llm_provider()`
3. ❌ Validation error is **caught but ignored** in chat handler
4. ❌ Handler logs warning and falls back to default provider silently
5. ❌ User sees OpenAI API error instead of helpful validation message

### Evidence from Code

**File:** [chat.rs](edgequake/crates/edgequake-api/src/handlers/chat.rs#L384-L394)

```rust
match ProviderFactory::create_llm_provider(&provider_name, &model_name) {
    Ok(llm) => {
        debug!("Created LLM provider override");
        (Some(llm), Some(provider_name), Some(model_name))
    }
    Err(e) => {
        warn!(error = %e, "Failed to create LLM provider, using default");
        (None, None, None)  // ❌ Silently falls back!
    }
}
```

### User Experience Issue

User explicitly selects "OpenAI" → Expects either:

- ✅ Query works with OpenAI
- ✅ Clear error: "OpenAI requires API key. Use Ollama/LMStudio instead"

But gets:

- ❌ Confusing OpenAI API error (implies request was sent)
- ❌ No indication that fallback occurred

---

## 🧭 ORIENT

### Root Cause Analysis

The chat handler has **two error handling strategies**:

1. **Provider Override Creation** (lines 384-394):

   - Silent fallback to default on error
   - Intent: Be resilient when provider unavailable
   - Problem: Masks configuration errors from user

2. **Query Execution** (lines 407-413):
   - Returns error to user
   - Intent: Report execution failures
   - Problem: Too late - already used wrong provider

### Why Silent Fallback Fails

- **In development**: User needs immediate feedback about misconfiguration
- **Explicit selection**: User chose OpenAI deliberately, fallback violates intent
- **Auto-detection**: Silent fallback makes sense
- **Explicit choice**: Silent fallback is confusing

### Decision Matrix

| Scenario                     | User Action                | Expected Behavior | Current Behavior   |
| ---------------------------- | -------------------------- | ----------------- | ------------------ |
| 1. No provider selected      | Uses server default        | Use Ollama        | ✅ Works           |
| 2. Selects valid provider    | Chooses Ollama             | Use Ollama        | ✅ Works           |
| 3. Selects invalid provider  | Chooses OpenAI without key | **Error message** | ❌ Silent fallback |
| 4. Provider fails at runtime | Query with OpenAI          | Error message     | ✅ Works           |

---

## 🎯 DECIDE

### Strategy

**Return validation errors to user for explicit provider selections**

### Implementation Plan

#### Change 1: Distinguish Intent

Add parameter to indicate explicit vs implicit provider selection:

```rust
struct ProviderIntent {
    explicit: bool,  // User explicitly selected vs auto-detected
    provider: String,
    model: String,
}
```

#### Change 2: Conditional Error Handling

```rust
if provider_intent.explicit {
    // User chose this provider - return validation error
    match create_llm_provider() {
        Ok(llm) => use_override,
        Err(e) => return Err(ApiError::BadRequest(e.to_string()))
    }
} else {
    // Auto-detected - fall back silently
    match create_llm_provider() {
        Ok(llm) => use_override,
        Err(e) => {
            warn!("Provider unavailable, using default");
            use_default
        }
    }
}
```

#### Change 3: Enhanced Error Messages

Validation errors already include helpful suggestions:

```
OPENAI_API_KEY is empty or invalid.
Provide a valid API key from https://platform.openai.com/account/api-keys
or select a different provider (ollama, lmstudio, mock)
```

### Alternative: Always Return Errors

**Simpler approach:** Never silently fall back when user provides provider parameter

Pros:

- ✅ Simpler code
- ✅ Clear user feedback
- ✅ No ambiguity about which provider was used

Cons:

- ⚠️ Breaks graceful degradation for auto-detection
- ⚠️ Requires frontend to handle provider selection more carefully

**Decision:** Use **Alternative approach** - if user provides `provider` parameter, always validate and return errors

---

## ⚡ ACT

### Files to Modify

1. **edgequake/crates/edgequake-api/src/handlers/chat.rs**
   - Lines ~384-394 (non-streaming handler)
   - Lines ~684-694 (streaming handler)
   - Change: Return validation errors instead of falling back silently

### Implementation Steps

1. ✅ Read current error handling code
2. ⏳ Modify non-streaming handler
3. ⏳ Modify streaming handler
4. ⏳ Test with explicit provider selection
5. ⏳ Verify error messages reach frontend
6. ⏳ Update SPEC-032 status

---

## 📊 Success Criteria

- [ ] User selects OpenAI without API key → Gets validation error (not API error)
- [ ] Error message suggests alternatives (ollama, lmstudio, mock)
- [ ] Error displays in UI query response
- [ ] Default provider still works when no explicit selection
- [ ] No regressions in existing provider selection

---

## 🔗 Related

- **Previous Loop:** OODA Loop 50 - Added OpenAI API key validation
- **Commits:** 93a2d5f, 4dcf46f, 2ae337f
- **Spec:** SPEC-032 - Multi-provider support
