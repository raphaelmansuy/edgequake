# Task Log: Reliable Ingestion Mission

**Date:** 2025-02-07
**Session:** reliable-ingestion-mission

## Actions
- Tested document upload via MCP Playwright (3 PDFs: national-capitals.pdf, Projet Loi de Finances 2026.pdf, Sommaire.pdf)
- Verified Knowledge Graph building (200 entities, 6 types, 11 connections)
- Updated all gpt-4o-mini references to gpt-5-nano in 6 files
- Added gpt-5-nano pricing to ModelPricing configuration
- Ran 340 tests (141 pipeline + 199 LLM) - all passed
- Rebuilt release binary (1m 27s)
- Restarted backend with proper environment variables

## Decisions
- Kept in-memory providers (legitimate for dev/test mode when DATABASE_URL not set)
- Used gpt-5-nano as OpenAI default (gpt-4o-mini quota exceeded)
- Estimated gpt-5-nano pricing at $0.00015/$0.0006 per 1K tokens (input/output)

## Next Steps
- Monitor production usage for gpt-5-nano pricing accuracy
- Consider adding retry logic for transient extraction failures
- Add more comprehensive E2E test coverage

## Lessons/Insights
- In-memory providers follow SRP correctly - only used as fallback when no DATABASE_URL
- Entity deduplication working well (~5% merge rate)
- Ollama with gemma3:latest provides good local entity extraction
