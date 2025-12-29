# Task Log: WebUI Ingestion Cost Display Implementation

**Date:** 2025-12-29  
**Mode:** Beastmode  
**Duration:** ~20 minutes

## Actions

- Added cost fields (`cost_usd`, `input_tokens`, `output_tokens`, `total_tokens`, `llm_model`, `embedding_model`) to backend DocumentSummary
- Updated processor.rs to store cost tracking in async processing metadata
- Added cost fields to frontend Document TypeScript interface
- Created new CostCell component with rich tooltip (cost-cell.tsx)
- Updated document-manager.tsx to use CostCell instead of duration proxy
- Added Processing Cost section to document-preview-panel.tsx
- Applied modern table styling: sticky header, alternating rows, smooth transitions
- Verified with browser testing: document upload, cost display, tooltip, preview panel

## Decisions

- Made Cost column visible on all screen sizes (removed lg:table-cell hiding)
- Centered Entities and Cost columns for better visual balance
- Used green color for costs < $0.001, blue for < $0.01, yellow for < $0.1, orange for >= $0.1
- Token counts show "0" when LLM doesn't return usage data (expected for some models)

## Next Steps

- Fix async processing to complete (some documents stuck in "Processing")
- Add cost estimation during upload (pre-processing estimate)
- Investigate token count population in OpenAI response
- Add historical cost charts to Costs page

## Lessons/Insights

- CostBadge was using a fake duration-based cost proxy; replaced with real API data
- OpenAI provider returns cost correctly but token counts may require additional parsing
- Fast Refresh in Next.js made iterative UI development efficient
- Browser testing with Playwright MCP provides reliable verification
