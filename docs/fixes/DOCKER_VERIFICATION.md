---
title: Docker deployment verification
description: Incident note from 2026-02-09: checks used after the frontend-backend Docker fix.
---

> Historical note, 2026-02-09; may not match current code.
> Prefer: [Docker quick start](DOCKER_QUICK_START.md) · [Troubleshooting](../troubleshooting/common-issues.md)

## ✅ Fixes Applied

### 1. Frontend-Backend Connectivity
**Problem**: Frontend showed "API Status: Disconnected" even though backend was running.

**Root Cause**: `NEXT_PUBLIC_API_URL` environment variable must be set at **build time** for Next.js, not just runtime.

**Solution**:
- Added `NEXT_PUBLIC_API_URL` as build ARG in `edgequake_webui/Dockerfile`
- Passed build arg in `edgequake/docker/docker-compose.yml`
- Frontend bundle now has correct backend URL baked in at build time

### 2. OpenAI API Key Inheritance
**Problem**: Backend needs to inherit OPENAI_API_KEY from host environment.

**Solution**:
- Updated docker-compose.yml: `OPENAI_API_KEY=${OPENAI_API_KEY:-}`
- Backend now inherits from host with fallback to empty string
- No need to rebuild - just restart with `make docker-up`

## 🧪 How to Test

### Step 1: Verify Services are Running
```bash
# Check all services status
cd /Users/raphaelmansuy/Github/03-working/edgequake && make docker-up

# Wait for services to start (20 seconds)
sleep 20

# Verify backend health
curl http://localhost:8080/health | python3 -m json.tool

# Expected output:
# {
#   "status": "healthy",
#   "llm_provider_name": "openai",
#   ...
# }
```

### Step 2: Access Frontend
1. **Open browser** to http://localhost:3000
2. **Refresh the page** (Cmd+Shift+R on macOS) to clear cache
3. **Verify** you see:
   - ✅ "API Status: Connected" (green)
   - ✅ "LLM Provider: OpenAI" (or the configured provider)

### Step 3: Create a Tenant and Test Query

#### 3.1 Create New Tenant
1. Click **"Create New Tenant"** button in the left sidebar
2. Enter tenant name (e.g., "test-tenant")
3. Click **Create**

#### 3.2 Upload a Document
1. Select the new tenant from the sidebar
2. Click **"Upload Documents"** in the Quick Actions section
3. Choose a PDF file (e.g., from `zz_test_docs/`)
4. Wait for processing to complete (status should show "Completed")

#### 3.3 Test Query with OpenAI
1. Navigate to **Query Knowledge** page
2. Enter a query related to your uploaded document
3. Submit the query
4. Verify:
   - Query response is generated using OpenAI
   - Entities are extracted and linked
   - Graph visualization shows relationships

### Step 4: Verify Environment Variables

```bash
# Check backend has OPENAI_API_KEY
docker exec edgequake env | grep OPENAI_API_KEY

# Should output your API key (first few characters)
```

## 🐛 Troubleshooting

### Frontend Still Shows "Disconnected"
**Solution**: Hard refresh the browser (Cmd+Shift+R) to clear cached JavaScript bundle.

### Backend Shows "llm_provider_name": "mock"
**Solution**: 
```bash
# Ensure OPENAI_API_KEY is set in your shell
export OPENAI_API_KEY="sk-your-key-here"

# Restart Docker stack
make docker-down && make docker-up
```

### CORS Errors in Browser Console
**Solution**: Backend CORS is already configured. If you see errors:
1. Check backend logs: `docker compose logs edgequake --tail=50`
2. Verify frontend is using correct API URL in Network tab (should be http://localhost:8080)

## 📊 What Changed

### Files Modified
1. **edgequake_webui/Dockerfile**
   - Added `ARG NEXT_PUBLIC_API_URL=http://localhost:8080` in builder stage
   - Set `ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}` before build

2. **edgequake/docker/docker-compose.yml**
   - Added build args for frontend service:
     ```yaml
     frontend:
       build:
         args:
           NEXT_PUBLIC_API_URL: http://localhost:8080
     ```
   - Updated backend environment:
     ```yaml
     environment:
       - OPENAI_API_KEY=${OPENAI_API_KEY:-}
     ```

## ✨ Expected Behavior

### Before Fix
- ❌ Frontend: "API Status: Disconnected"
- ❌ Frontend: "LLM Provider: Unavailable"
- ❌ API calls failing silently
- ❌ No query functionality

### After Fix
- ✅ Frontend: "API Status: Connected" (green)
- ✅ Frontend: "LLM Provider: OpenAI" or configured provider
- ✅ All API endpoints accessible
- ✅ Document upload works
- ✅ Query with OpenAI works
- ✅ Entity extraction works
- ✅ Graph visualization works

## 🔍 Quick Verification Script

Save this as `verify_docker.sh` and run it:

```bash
#!/bin/bash

echo "🔍 EdgeQuake Docker Verification"
echo "================================"
echo ""

# 1. Check frontend
echo "1️⃣ Checking frontend..."
FRONTEND_STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000)
if [ "$FRONTEND_STATUS" = "200" ]; then
  echo "   ✅ Frontend responding (HTTP 200)"
else
  echo "   ❌ Frontend not accessible (HTTP $FRONTEND_STATUS)"
  exit 1
fi

# 2. Check backend
echo "2️⃣ Checking backend..."
BACKEND_STATUS=$(curl -s http://localhost:8080/health | python3 -c "import sys, json; print(json.load(sys.stdin)['status'])")
if [ "$BACKEND_STATUS" = "healthy" ]; then
  echo "   ✅ Backend healthy"
else
  echo "   ❌ Backend not healthy"
  exit 1
fi

# 3. Check LLM provider
echo "3️⃣ Checking LLM provider..."
LLM_PROVIDER=$(curl -s http://localhost:8080/health | python3 -c "import sys, json; print(json.load(sys.stdin)['llm_provider_name'])")
echo "   ℹ️  Provider: $LLM_PROVIDER"

# 4. Check OPENAI_API_KEY in backend
echo "4️⃣ Checking OPENAI_API_KEY in backend..."
if docker exec edgequake env | grep -q OPENAI_API_KEY; then
  echo "   ✅ OPENAI_API_KEY is set"
else
  echo "   ⚠️  OPENAI_API_KEY not set (using default provider)"
fi

echo ""
echo "================================"
echo "✅ All basic checks passed!"
echo ""
echo "📝 Next steps:"
echo "   1. Open http://localhost:3000 in your browser"
echo "   2. Refresh the page (Cmd+Shift+R)"
echo "   3. Create a new tenant"
echo "   4. Upload a document"
echo "   5. Test queries"
```

## 📅 Date: February 9, 2026

**Status**: ✅ VERIFIED - All services running correctly with proper connectivity.
