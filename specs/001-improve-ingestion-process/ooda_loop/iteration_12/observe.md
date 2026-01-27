# Observe - Iteration 12: Rebuild KG Flow Verification

## User Objective
"Ensure Rebuild KG works -> in case of Rebuild KG you must re-extract and rebuild embedding"

## Current Implementation Status
Need to verify:
1. Backend API endpoint for Rebuild KG
2. What operations it performs
3. Whether it re-extracts entities AND rebuilds embeddings
4. Frontend triggers the correct endpoint

## Files to Examine
- Backend: `edgequake-api/src/handlers/` - Find rebuild KG handler
- Frontend: `rebuild-knowledge-graph-button.tsx` - What API does it call?

## Expected Flow for "Rebuild KG"
1. Clear existing graph nodes/edges for workspace
2. For each document:
   a. Re-run entity extraction (LLM)
   b. Re-run embedding generation
   c. Store in graph database
   d. Store in vector database

## Questions to Answer
1. Does the API support this full rebuild?
2. Is there a flag for "full rebuild" vs "partial"?
3. Does the frontend show clear confirmation before rebuild?

## Next Step
Read the backend rebuild KG endpoint and frontend button implementation
