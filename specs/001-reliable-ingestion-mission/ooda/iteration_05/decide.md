# OODA Iteration 05 - Decide

## Decision: Create Comprehensive Dev Mode Documentation

### 1. Selected Action

**Add "Developer Workflow Guide" section to AGENTS.md**

This addresses the mission requirement:
> "Document the best way to run EdgeQuake in dev mode during testing session."

### 2. Why AGENTS.md?

Per project conventions:
- AGENTS.md is the primary instruction file for automated agents
- Already has "Quick Start with make" section
- Referenced in skills and README
- Most discoverable location for dev workflow

### 3. Documentation Structure

Will add/update the following sections:

#### A. Prerequisites Section (Update)
- PostgreSQL required (no memory mode)
- Ollama for local LLM
- pdfium library for PDF extraction

#### B. Developer Workflow (NEW)
- Complete step-by-step for new developers
- Service startup sequence
- Verification commands
- Common troubleshooting

#### C. LLM Provider Configuration (NEW)
- Ollama setup (default)
- OpenAI setup (alternative)
- gpt-5-nano migration note
- Runtime provider switching

#### D. Testing Workflow (NEW)
- Running specific test suites
- Verifying changes
- Full test verification

### 4. Content Outline

```markdown
## Developer Workflow

### Quick Start (Recommended)
1. Start services: `make dev`
2. Verify health: `make status`
3. Open UI: http://localhost:3000

### Prerequisites Checklist
- [ ] Docker installed (for PostgreSQL)
- [ ] Ollama installed (for LLM)
- [ ] Rust toolchain (for building)
- [ ] Node.js/pnpm (for frontend)

### Service Health Verification
[commands and expected outputs]

### LLM Provider Configuration
[Ollama vs OpenAI, gpt-5-nano recommendation]

### Common Issues & Solutions
[troubleshooting guide]
```

### 5. Changes Required

| File | Action | Location |
|------|--------|----------|
| AGENTS.md | Add Developer Workflow section | After "Quick Start with make" |
| AGENTS.md | Update prerequisites | Before workflow section |
| AGENTS.md | Add troubleshooting | End of document |

### 6. Validation Criteria

Documentation will be successful if:
1. ✅ New developer can follow steps to start services
2. ✅ Health verification commands are documented
3. ✅ LLM choices are clearly explained
4. ✅ Common issues have documented solutions
5. ✅ Testing workflow is clear

### 7. Risk Assessment

| Risk | Mitigation |
|------|------------|
| Documentation becomes stale | Keep close to Makefile targets |
| Too verbose | Use tables and code blocks |
| Missing edge cases | Based on mission experience |

### 8. Decision Rationale

Why documentation now?
1. Code is stable (641+ tests passing)
2. Mission requires it explicitly
3. Knowledge is fresh from troubleshooting
4. Single coherent location (AGENTS.md)

### 9. Alternative Approaches Rejected

| Approach | Why Rejected |
|----------|--------------|
| Separate docs/dev-guide.md | Less discoverable |
| Update README.md | Too high-level |
| Create new SKILL | Overhead for internal docs |
| Update existing skill | No matching skill exists |

## Decision Confirmed

**Action:** Add comprehensive "Developer Workflow" section to AGENTS.md, incorporating all learnings from OODA iterations 01-04.

**Commit message:** `OODA-05: Add dev mode workflow documentation`
