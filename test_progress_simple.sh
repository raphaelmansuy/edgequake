#!/bin/bash
# Simplified test: Upload PDF and tail logs to see progress updates
# Tests OODA-PERF-01/02 optimizations

set -e

PDF_PATH="/Users/raphaelmansuy/Github/03-working/edgequake/zz-explore/agentfail_2601.22984v1.pdf"
API_URL="http://localhost:8080"
TENANT_ID="test-tenant"
WORKSPACE_ID="default"

echo "🚀 Testing OODA-PERF-01/02: Progress Optimization"
echo "=================================================="
echo ""
echo "PDF: agentfail_2601.22984v1.pdf (39 pages, 1.6MB)"
echo "Expected:"
echo "  - PDF extraction: ~8 progress updates (every 5 pages)"
echo "  - Entity extraction: ~10 progress updates (every 3 chunks)"
echo "  - No silent periods > 3 seconds"
echo ""

# Clear old backend logs to see fresh output
echo "📋 Clearing old logs..."
> /tmp/edgequake-backend.log

# Start tailing logs in background
tail -f /tmp/edgequake-backend.log | grep -E "Converting PDF|Extracting entities|page_num|chunk_index|stage_message" &
TAIL_PID=$!

# Give tail a moment to start
sleep 1

# Upload PDF
echo "📄 Uploading PDF (this will take ~30 seconds)..."
echo ""

curl -X POST "$API_URL/api/v1/pdf/upload" \
  -H "Content-Type: multipart/form-data" \
  -H "X-Tenant-ID: $TENANT_ID" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "file=@$PDF_PATH" \
  -F "enable_vision=false" \
  -o /tmp/upload_response.json \
  2>&1 | cat

echo ""
echo "✅ Upload request sent"
echo ""
echo "📊 Watching logs for progress updates..."
echo "   (Press Ctrl+C when done)"
echo ""

# Wait for user to stop
wait $TAIL_PID
