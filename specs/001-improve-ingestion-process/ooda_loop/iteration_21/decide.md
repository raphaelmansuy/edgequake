# Iteration 21: Decide

## Action Plan

### Step 1: Add QueueMetrics Type

File: `edgequake_webui/src/types/index.ts`

- Add QueueMetrics interface after EnhancedPipelineStatus

### Step 2: Add getQueueMetrics API Function

File: `edgequake_webui/src/lib/api/edgequake.ts`

- Add function following getEnhancedPipelineStatus pattern

### Step 3: Create QueueMetricsCard Component

File: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

- Add new card component
- Use useQuery with 3s refetch interval
- Display worker gauge, metrics tiles, status footer

### Step 4: Integrate into PipelineMonitor Layout

File: `edgequake_webui/src/components/pipeline/pipeline-monitor.tsx`

- Add QueueMetricsCard to the layout grid

### Step 5: Validate

- Run TypeScript compilation
- Visual verification (if backend running)

## Priority

- HIGH: Core Objective B requirement
- Dependencies: Iterations 19-20 complete ✅
