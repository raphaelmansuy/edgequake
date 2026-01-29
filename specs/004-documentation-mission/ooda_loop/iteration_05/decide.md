# OODA Iteration 05 - Decide

**Date**: 2025-01-XX
**Focus**: REST API Reference Documentation

## 📋 Decision

### Selected Deliverable

**Primary Output**: `docs/api-reference/rest-api.md`

A comprehensive REST API reference (~600 lines) covering:

1. **Authentication Section**
   - JWT Bearer tokens
   - API Key header (X-API-Key)
   - Multi-tenant headers

2. **Core Endpoints (detailed)**
   - Documents API (CRUD, processing status)
   - Query API (single + streaming)
   - Chat API (OpenAI-compatible)
   - Graph API (entities, relationships, stats)

3. **Supporting Endpoints (summary)**
   - Health & metrics
   - Workspaces & tenants
   - Conversations
   - Models & settings

4. **Error Handling**
   - RFC 7807 Problem Details format
   - Common error codes
   - Retry strategies

### Format Decisions

- Use tables for endpoint summaries
- Include cURL examples for all core endpoints
- Add JSON schema excerpts for request/response bodies
- Include ASCII sequence diagrams for complex flows

### Success Criteria

- [ ] All 80+ endpoints documented
- [ ] Core endpoints have cURL examples
- [ ] Authentication clearly explained
- [ ] Error handling documented
- [ ] Streaming endpoints explained
