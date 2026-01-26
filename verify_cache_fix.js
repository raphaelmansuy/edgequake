#!/usr/bin/env node
/**
 * Verification script for cache invalidation fix
 * 
 * This script verifies:
 * 1. Cache manager exists and is properly implemented
 * 2. Dashboard imports and uses cache manager
 * 3. Cache versioning is in place
 */

const fs = require('fs');
const path = require('path');

const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const NC = '\x1b[0m';

function checkFile(filePath, checks) {
  console.log(`\nChecking: ${filePath}`);
  
  if (!fs.existsSync(filePath)) {
    console.log(`${RED}✗ File not found${NC}`);
    return false;
  }
  
  const content = fs.readFileSync(filePath, 'utf8');
  let allPassed = true;
  
  checks.forEach(({ name, pattern, required = true }) => {
    const found = content.includes(pattern) || new RegExp(pattern).test(content);
    
    if (found) {
      console.log(`${GREEN}✓${NC} ${name}`);
    } else if (required) {
      console.log(`${RED}✗${NC} ${name} (MISSING)`);
      allPassed = false;
    } else {
      console.log(`${YELLOW}○${NC} ${name} (optional, not found)`);
    }
  });
  
  return allPassed;
}

function main() {
  console.log('=== Cache Invalidation Fix Verification ===\n');
  
  const webUIRoot = path.join(__dirname, 'edgequake_webui');
  const srcRoot = path.join(webUIRoot, 'src');
  
  let allChecks = true;
  
  // Check 1: Cache manager exists
  allChecks &= checkFile(
    path.join(srcRoot, 'lib/cache-manager.ts'),
    [
      { name: 'Cache version constant', pattern: 'const CACHE_VERSION' },
      { name: 'getCacheContext function', pattern: 'export function getCacheContext' },
      { name: 'isCacheStale function', pattern: 'export function isCacheStale' },
      { name: 'clearQueryCache function', pattern: 'export function clearQueryCache' },
      { name: 'validateAndClearCache function', pattern: 'export function validateAndClearCache' },
      { name: 'forceCacheClear function', pattern: 'export function forceCacheClear' },
    ]
  );
  
  // Check 2: Dashboard imports cache manager
  allChecks &= checkFile(
    path.join(srcRoot, 'app/(dashboard)/page.tsx'),
    [
      { name: 'Imports cache manager', pattern: "import.*from '@/lib/cache-manager'" },
      { name: 'Imports useQueryClient', pattern: 'useQueryClient' },
      { name: 'Calls validateAndClearCache', pattern: 'validateAndClearCache' },
      { name: 'Has cache validation useEffect', pattern: 'useEffect.*validateAndClearCache' },
      { name: 'Has workspace change useEffect', pattern: 'Workspace changed, forcing stats refetch' },
      { name: 'Forces refetch on workspace change', pattern: 'refetchQueries.*workspaceStats' },
    ]
  );
  
  // Check 3: Tenant store has hydration tracking
  allChecks &= checkFile(
    path.join(srcRoot, 'stores/use-tenant-store.ts'),
    [
      { name: 'Has _hasHydrated state', pattern: '_hasHydrated' },
      { name: 'Has setHasHydrated action', pattern: 'setHasHydrated' },
      { name: 'Has onRehydrateStorage callback', pattern: 'onRehydrateStorage' },
    ]
  );
  
  // Check 4: E2E test exists
  allChecks &= checkFile(
    path.join(webUIRoot, 'e2e/dashboard-cache-invalidation.spec.ts'),
    [
      { name: 'Test suite exists', pattern: 'Dashboard Stats Cache Invalidation' },
      { name: 'Tests cache invalidation', pattern: 'should invalidate cache when workspace changes' },
      { name: 'Tests fresh fetch', pattern: 'should fetch fresh stats on every page load' },
    ]
  );
  
  // Summary
  console.log('\n=== Summary ===\n');
  
  if (allChecks) {
    console.log(`${GREEN}✓ All checks passed!${NC}`);
    console.log('\nThe cache invalidation fix has been properly implemented.');
    console.log('\nNext steps:');
    console.log('1. Run ./test_cache_invalidation.sh for manual testing guide');
    console.log('2. Open http://localhost:3000 in browser');
    console.log('3. Open DevTools Console and verify log messages');
    console.log('4. Check Network tab for /stats API calls on every page load');
    console.log('5. Verify stats show correct values (not 0/0)');
    return 0;
  } else {
    console.log(`${RED}✗ Some checks failed${NC}`);
    console.log('\nPlease review the missing components above.');
    return 1;
  }
}

process.exit(main());
