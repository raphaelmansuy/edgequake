# Iteration 22: Decide

## Action Plan

### Step 1: Enhance TaskQueueCard

File: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

Changes:

1. Add formatWaitTime helper function
2. Split tasks into pending and processing
3. Show queue position for pending tasks
4. Display wait time per task
5. Add icons for visual clarity

### Step 2: Add useMemo for task filtering

- Filter pending tasks (sorted by created_at ascending)
- Filter processing tasks

### Step 3: Update UI structure

- Section for pending tasks with queue position
- Section for processing tasks

### Step 4: Validate

- TypeScript compilation
- Visual inspection (when server running)

## Priority

- HIGH: Core Objective B requirement
- No backend changes needed
