# OODA-15: Orient - Add Subsection Pattern Tests

## Analysis

### Functions to Test

1. **`is_letter_subsection_item`**
   - Patterns: "A. Background", "B. Policy Representations"
   - Must have: letter + ". " + uppercase-heavy text
   - IEEE-style papers use this for subsections

2. **`is_number_section_header`**
   - Patterns: "1. Introduction", "2 Methods"
   - Must have: digit + (". " or " ") + all-caps text
   - ICML/NeurIPS-style papers use this

3. **`is_number_subsection_item`**
   - Patterns: "2.1. Agentic Training", "3.2 Architecture"
   - Must have: digit + "." + digit + (". " or " ")
   - Common in many paper formats

### Test Cases

| Function          | Valid Cases     | Invalid Cases                |
| ----------------- | --------------- | ---------------------------- |
| letter_subsection | "A. Background" | "A.NoSpace", "ABC. Too long" |
| number_section    | "1. INTRO"      | "1.1. Subsection"            |
| number_subsection | "2.1. Topic"    | "2. Main section"            |

## Prioritization

All three need tests - add a single test function covering all patterns.
