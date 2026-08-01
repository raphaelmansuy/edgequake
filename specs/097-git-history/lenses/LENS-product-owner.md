# Lens — Product Owner

## Done means

1. Contributors can clone EdgeQuake without downloading multi-GB regenerable bench JSON.  
2. Acc / latency **claims** remain auditable via thin `publish/` + `history/*/scorecard.json`.  
3. Future bench runs cannot re-bloat the remote (ignore + CI guard).  
4. [#351](https://github.com/raphaelmansuy/edgequake/issues/351) closed with before/after metrics.

## Non-goals

- Changing Acc methodology or peer semantics.  
- Hosting fat archives on LFS/S3 in this SPEC.  
- Deleting operators’ local `history/` forensics.
