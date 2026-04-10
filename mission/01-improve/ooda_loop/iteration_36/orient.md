# OODA-36 — Orient

Pure helper functions in merger/mod.rs have edge cases not covered by existing tests. MergerConfig default values, MergeStats zero state, and normalize_entity_name boundary inputs (unicode, tabs, single char) are all testable without mocking.
