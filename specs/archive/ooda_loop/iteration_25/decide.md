# Iteration 25 – DECIDE

## Decision

Create RebuildPhaseIndicator component with visual phase stepper.

## Implementation Plan

### 1. Add RebuildPhaseIndicator Component

Location: `edgequake_webui/src/components/documents/pipeline-status-dialog.tsx`

```tsx
interface RebuildPhase {
  id: string;
  label: string;
  description: string;
  status: 'complete' | 'active' | 'pending';
}

function RebuildPhaseIndicator({
  jobName,
  processedDocs,
  totalDocs,
  isBusy
}: {
  jobName?: string;
  processedDocs: number;
  totalDocs: number;
  isBusy: boolean;
}) {
  // Detect rebuild type
  const isKgRebuild = jobName?.startsWith('rebuild_kg_');
  const isEmbedRebuild = jobName?.startsWith('rebuild_embed_');

  if (!isKgRebuild && !isEmbedRebuild) return null;

  // Calculate phases based on progress
  const phases = isKgRebuild ? [
    { id: 'clear', label: 'Clear', description: 'Clearing entities' },
    { id: 'extract', label: 'Extract', description: 'Re-extracting' },
    { id: 'embed', label: 'Embed', description: 'Re-embedding' },
  ] : [
    { id: 'clear', label: 'Clear', description: 'Clearing vectors' },
    { id: 'embed', label: 'Embed', description: 'Re-embedding' },
  ];

  // Determine current phase
  // Clear is always done when we see status
  // Extract is active while processing
  // Embed is last phase (for KG)
  const currentPhase = ...;

  return (
    <div>
      <Stepper phases={phases} currentPhase={currentPhase} />
    </div>
  );
}
```

### 2. Visual Design

Use horizontal stepper with:

- Circles connected by lines
- Green check for complete phases
- Blue pulsing for active phase
- Gray for pending phases
- Phase labels below circles

### 3. Integration

Add to PipelineStatusDialog after job info, before progress bar.

## Success Criteria

- [ ] Rebuild operations show phase stepper
- [ ] Current phase is visually highlighted
- [ ] Completed phases show checkmarks
- [ ] Non-rebuild operations don't show indicator
