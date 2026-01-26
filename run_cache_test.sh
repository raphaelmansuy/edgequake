#!/usr/bin/env bash
# Automated browser test for cache invalidation
# Uses Playwright to verify the fix works

set -e

cd /Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui

echo "=== Running Dashboard Cache Invalidation E2E Test ==="
echo ""

# Run the E2E test
npx playwright test e2e/dashboard-cache-invalidation.spec.ts --headed --project=chromium

echo ""
echo "Test completed!"
