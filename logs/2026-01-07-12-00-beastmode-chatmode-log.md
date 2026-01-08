Actions: Refactored orchestrator to SOTA engine, hit cyclic dep; reverted to core QueryEngine; added query unit test; updated roadblockers with architectural plan; ran core build/tests.
Decisions: Avoid cyclic dependency; keep core QueryEngine for now; document path forward.
Next steps: Design trait crate for QueryService; plan engine injection in API; schedule dead-code removal after refactor.
Lessons/insights: Workspace cycles block naive swaps; add guard tests before large refactors.
