# Lens — DevOps / Git

## Rules

- Tip cleanup without rewrite is incomplete (LAW-G4).  
- Use `git-filter-repo` in a mirror clone; never rewrite casually on a dirty shared worktree.  
- Force-push requires re-clone announcement (EC-01, EC-02).  
- `make git-hygiene` runs on every CI push/PR (LAW-G5).

## Ops checklist

1. Backup mirror.  
2. Filter paths (runbook).  
3. Verify blob sums.  
4. `--force --mirror` push.  
5. Notify PR authors.  
6. Expect GitHub size UI lag (EC-03).
