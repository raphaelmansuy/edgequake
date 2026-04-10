# OODA-17: Orient — Edge Case Tests for Row Types

## First Principles Analysis

Functions needing edge case tests:

### `normalize_entity_types`
- Empty input → empty output
- All-whitespace entries → filtered out
- Case normalization: "Person" → "PERSON"
- Space/hyphen replacement: "key person" → "KEY_PERSON"
- Deduplication: ["Person", "PERSON"] → ["PERSON"]
- Max 50 types cap
- Unicode characters (should be preserved)

### `parse_plan`
- Known plans: "basic", "pro", "enterprise"
- Case insensitive: "Pro" → Pro
- Unknown → Free
- Empty string → Free

### `parse_role`
- Known roles: "readonly", "admin", "owner"  
- Case insensitive: "Admin" → Admin
- Unknown → Member
- Empty string → Member
