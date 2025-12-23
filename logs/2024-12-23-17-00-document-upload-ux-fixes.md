# Task Log: Document Upload UX Fixes

**Date**: 2024-12-23  
**Commit**: f37b623

## Actions

- Audited document upload UX based on user screenshots
- Identified root causes: async processing not working, docs without chunks not listed
- Switched frontend to sync processing (async_processing: false)
- Fixed backend to include documents without chunks in list
- Disabled BatchProgressCard that was causing infinite spinner
- All tests pass (19 API, 18 E2E)

## Decisions

- Use synchronous processing until async TaskProcessor is properly implemented
- Keep BatchProgressCard code commented for future async implementation
- Sort documents by created_at descending for newest first

## Next Steps

- Implement real TaskProcessor to enable async document processing
- Re-enable BatchProgressCard when async processing works
- Add real-time WebSocket updates for processing status

## Lessons/Insights

- Document listing was filtering by chunk presence, missing pending docs
- Async tasks were queued but never consumed (no worker processing them)
- Sync processing path works correctly and updates status to "completed"
