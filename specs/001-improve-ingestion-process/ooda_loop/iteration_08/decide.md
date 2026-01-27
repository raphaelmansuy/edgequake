# Iteration 08 - DECIDE Phase

## Decision: Add ETA to Pipeline Status Dialog

### Implementation Plan

1. Calculate processing rate based on:
   - `job_start` timestamp
   - `processed_documents` count
   - Current time

2. Calculate ETA:
   - Remaining = total - processed
   - Rate = processed / elapsed_minutes
   - ETA = remaining / rate

3. Display ETA in progress section

### Code Changes

In `pipeline-status-dialog.tsx`:
```tsx
// Calculate ETA
const calculateEta = () => {
  if (!data?.job_start || !data.processed_documents) return null;
  
  const startTime = new Date(data.job_start).getTime();
  const now = Date.now();
  const elapsedMs = now - startTime;
  const elapsedMinutes = elapsedMs / 60000;
  
  if (elapsedMinutes < 0.5) return 'Calculating...';
  
  const rate = data.processed_documents / elapsedMinutes;
  const remaining = data.total_documents - data.processed_documents;
  const etaMinutes = remaining / rate;
  
  if (etaMinutes < 1) return 'Less than a minute';
  if (etaMinutes < 60) return `~${Math.ceil(etaMinutes)} minutes`;
  return `~${Math.ceil(etaMinutes / 60)} hours`;
};
```

### Display Location

Add after progress percentage:
```tsx
{eta && (
  <span className="text-muted-foreground">
    ETA: {eta}
  </span>
)}
```
