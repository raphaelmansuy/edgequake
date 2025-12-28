# Build Crash Investigation - Task Log

**Date**: 2024-12-27 16:27 (beastmode)
**Issue**: VS Code window crashes during Next.js build with 100% CPU

## Root Cause Analysis

The crash was **NOT** caused by TypeScript type complexity. Investigation revealed:

1. **TypeScript check passes in 3.28s** - No circular dependencies or complex type inference issues
2. **Next.js build completes in ~8s** when run in isolation
3. **VS Code Plugin Helper at 126% CPU** - The extension host was overloaded
4. **15 workers spawned** by Next.js competing with VS Code TS server for resources

The crash ("crashed, code: 5" = SIGTRAP) was caused by **resource exhaustion** when:

- Next.js spawned 15 parallel workers for static page generation
- VS Code's TypeScript language server was running simultaneously
- Combined memory/CPU pressure exceeded system limits

## Actions Taken

1. **Created `.vscode/settings.json`** with optimizations:

   - Limited TS server memory to 3GB
   - Enabled separate syntax server
   - Excluded heavy folders from file watching
   - Optimized ESLint to run only on save

2. **Updated `next.config.ts`**:

   - Limited workers to 4 (down from 15)
   - Added webpack build worker
   - Set standalone output mode

3. **Created `scripts/safe-build.sh`**:

   - Cleans caches before build
   - Runs with `nice -n 10` (lower priority)
   - Sets Node.js memory limit to 4GB
   - Includes timeout protection (5 minutes)
   - Runs TypeScript check first

4. **Added npm scripts**:
   - `npm run build:safe` - Full safe build with monitoring
   - `npm run build:check` - TypeScript check only
   - `npm run typecheck` - Quick type verification

## Verification Results

- Build now uses **4 workers** instead of 15
- Build completes in **4.4 seconds** (was timing out)
- No TypeScript errors
- All static pages generated successfully

## Recommendations

1. Use `npm run build:safe` for builds if crashes persist
2. Close other heavy VS Code windows during builds
3. Consider upgrading Node.js memory if needed: `export NODE_OPTIONS="--max-old-space-size=8192"`

## Files Changed

- `edgequake_webui/.vscode/settings.json` (created)
- `edgequake_webui/next.config.ts` (updated)
- `edgequake_webui/scripts/safe-build.sh` (created)
- `edgequake_webui/package.json` (updated scripts)

---

## Task Logs

- **Actions**: Analyzed TypeScript config, checked circular deps (none), created VS Code settings, limited Next.js workers, created safe build script
- **Decisions**: Root cause is resource exhaustion not type complexity; limit workers to 4; add memory limits
- **Next steps**: Monitor builds for stability; consider CI-specific worker limits
- **Lessons**: Next.js 16 defaults to CPU count for workers which can overload VS Code; file watcher limits critical for large projects
