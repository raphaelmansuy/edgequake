#!/bin/bash
# Test script to upload PDF and monitor progress
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
echo "Expected behavior:"
echo "  1. PDF extraction: Updates every 5 pages (8 updates)"
echo "  2. Entity extraction: Updates every 3 chunks (10 updates)"
echo "  3. No silent periods > 3 seconds"
echo ""

# Step 1: Upload PDF
echo "📄 Uploading PDF..."
UPLOAD_RESPONSE=$(curl -s -X POST "$API_URL/api/v1/pdf/upload" \
  -H "Content-Type: multipart/form-data" \
  -H "X-Tenant-ID: $TENANT_ID" \
  -H "X-Workspace-ID: $WORKSPACE_ID" \
  -F "file=@$PDF_PATH" \
  -F "enable_vision=false")

echo "Upload response: $UPLOAD_RESPONSE"

# Extract PDF ID and Document ID
PDF_ID=$(echo "$UPLOAD_RESPONSE" | jq -r '.pdf_id // empty')
TASK_ID=$(echo "$UPLOAD_RESPONSE" | jq -r '.task_id // empty')

if [ -z "$PDF_ID" ]; then
  echo "❌ Failed to extract PDF ID from response"
  echo "Full response: $UPLOAD_RESPONSE"
  exit 1
fi

echo "✅ PDF uploaded successfully"
echo "   PDF ID: $PDF_ID"
echo "   Task ID: $TASK_ID"
echo ""

# Step 2: Monitor progress by polling documents
echo "📊 Monitoring progress (polling every 1 second)..."
echo "────────────────────────────────────────────────────────────────"

START_TIME=$(date +%s)
LAST_MESSAGE=""
UPDATE_COUNT=0
POLL_COUNT=0

while true; do
  POLL_COUNT=$((POLL_COUNT + 1))
  CURRENT_TIME=$(date +%s)
  ELAPSED=$((CURRENT_TIME - START_TIME))
  
  # Poll documents endpoint
  DOCS_RESPONSE=$(curl -s -X GET "$API_URL/api/v1/documents" \
    -H "X-Tenant-ID: $TENANT_ID" \
    -H "X-Workspace-ID: $WORKSPACE_ID")
  
  # Find our document by PDF ID
  DOC=$(echo "$DOCS_RESPONSE" | jq -r --arg pdf_id "$PDF_ID" \
    '.documents[] | select(.pdf_id == $pdf_id) // empty')
  
  if [ -z "$DOC" ]; then
    echo "[${ELAPSED}s] Waiting for document to appear..."
    sleep 1
    continue
  fi
  
  # Extract progress fields
  STATUS=$(echo "$DOC" | jq -r '.status // "unknown"')
  STAGE=$(echo "$DOC" | jq -r '.current_stage // "unknown"')
  MESSAGE=$(echo "$DOC" | jq -r '.stage_message // "No message"')
  PROGRESS=$(echo "$DOC" | jq -r '.stage_progress // 0')
  
  # Check if message changed
  if [ "$MESSAGE" != "$LAST_MESSAGE" ]; then
    UPDATE_COUNT=$((UPDATE_COUNT + 1))
    PROGRESS_PCT=$(echo "$PROGRESS * 100" | bc | cut -d. -f1)
    echo "[${ELAPSED}s] $MESSAGE (progress: ${PROGRESS_PCT}%)"
    LAST_MESSAGE="$MESSAGE"
  fi
  
  # Check if processing complete
  if [ "$STATUS" = "completed" ] || [ "$STATUS" = "failed" ]; then
    echo ""
    if [ "$STATUS" = "completed" ]; then
      echo "✅ Processing complete!"
    else
      ERROR_MSG=$(echo "$DOC" | jq -r '.error_message // "Unknown error"')
      echo "❌ Processing failed: $ERROR_MSG"
    fi
    echo ""
    echo "📊 Statistics:"
    echo "   Total time: ${ELAPSED}s"
    echo "   Progress updates: $UPDATE_COUNT"
    echo "   Polls: $POLL_COUNT"
    echo "   Average update frequency: $((ELAPSED / UPDATE_COUNT))s"
    break
  fi
  
  # Timeout after 60 seconds
  if [ $ELAPSED -gt 60 ]; then
    echo ""
    echo "⏰ Timeout after 60 seconds"
    echo "   Last status: $STATUS"
    echo "   Last stage: $STAGE"
    echo "   Last message: $MESSAGE"
    exit 1
  fi
  
  sleep 1
done

echo ""
echo "✅ Test complete!"
