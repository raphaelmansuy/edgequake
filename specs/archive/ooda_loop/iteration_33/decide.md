# Iteration 33: Decide

## Decision

No code changes. The placeholder retry handlers are acceptable.

## Rationale

1. The mutations require a `documentId` parameter not available in error handler scope
2. Users have an alternative retry path via document row actions
3. The error toast still provides context about what failed

## Accepted Limitations

| Mutation          | Limitation  | Alternative                     |
| ----------------- | ----------- | ------------------------------- |
| deleteMutation    | Empty retry | Click delete on document row    |
| reprocessMutation | Empty retry | Click reprocess on document row |
