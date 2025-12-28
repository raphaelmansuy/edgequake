#!/bin/bash

# Test non-streaming query to check token count

TENANT_ID="cbbef300-8906-4d0c-a00a-1f09c084d6c2"
USER_ID="00000000-0000-0000-0000-000000000001"
API_URL="http://localhost:8080/api"

echo "=== Testing Non-Streaming Query ==="
echo ""

# Test query
RESPONSE=$(curl -s "$API_URL/chat/completions" \
  -X POST \
  -H "X-Tenant-ID: $TENANT_ID" \
  -H "X-User-ID: $USER_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What is Python?",
    "mode": "naive",
    "stream": false
  }')

echo "Response:"
echo "$RESPONSE" | jq '.tokens_used, .duration_ms, .content | .[0:100]' 2>/dev/null

# Also check the conversation message
echo ""
echo "=== Checking Conversation Messages ==="

CONV_ID=$(echo "$RESPONSE" | jq -r '.conversation_id' 2>/dev/null)
if [ ! -z "$CONV_ID" ] && [ "$CONV_ID" != "null" ]; then
  echo "Conversation ID: $CONV_ID"
  
  CONV=$(curl -s "$API_URL/conversations/$CONV_ID" \
    -H "X-Tenant-ID: $TENANT_ID" \
    -H "X-User-ID: $USER_ID")
  
  echo "Messages:"
  echo "$CONV" | jq '.messages[] | {role, tokens_used, duration_ms, content: (.content | .[0:50])}' 2>/dev/null
else
  echo "Failed to get conversation ID"
  echo "Full response:"
  echo "$RESPONSE" | jq '.' 2>/dev/null
fi
