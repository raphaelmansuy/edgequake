# Iteration 61 - DECIDE Phase

## Decision: Add WebUI Business Rules and Use Cases

### Plan

1. **Add BR06XX to business_rules.md** ✅

   - 12 WebUI-specific business rules
   - Reference actual store files (kebab-case)
   - Cross-reference FEAT06XX

2. **Add UC06XX to use_cases.md** ✅

   - 10 WebUI user journey use cases
   - Reference component paths
   - Link to BR06XX and FEAT06XX

3. **Fix store references** ✅
   - Replace CamelCase with kebab-case filenames
   - Validate against actual file listing

### Rules to Add (BR0601-BR0612)

| ID     | Rule Name                          | Related Feature    |
| ------ | ---------------------------------- | ------------------ |
| BR0601 | Theme Persistence                  | FEAT0619           |
| BR0602 | Conversation History Persistence   | FEAT0610, FEAT0613 |
| BR0603 | Graph Node Display Limits          | FEAT0601, FEAT0602 |
| BR0604 | Streaming State Transitions        | FEAT0609, FEAT0611 |
| BR0605 | Keyboard Navigation                | FEAT0618           |
| BR0606 | Document Upload Size Limit         | FEAT0605           |
| BR0607 | API Error Display                  | FEAT0615           |
| BR0608 | Settings Validation                | FEAT0608           |
| BR0609 | Real-time Sync Conflict Resolution | FEAT0616           |
| BR0610 | Modal Focus Trap                   | FEAT0618           |
| BR0611 | Query History Limit                | FEAT0610           |
| BR0612 | Loading State Feedback             | FEAT0617           |

### Use Cases to Add (UC0601-UC0610)

| ID     | Use Case Name               | Primary Actor |
| ------ | --------------------------- | ------------- |
| UC0601 | Visualize Knowledge Graph   | WebUI User    |
| UC0602 | Execute RAG Query           | WebUI User    |
| UC0603 | Upload Document via UI      | WebUI User    |
| UC0604 | Manage Conversation History | WebUI User    |
| UC0605 | Switch Theme                | WebUI User    |
| UC0606 | Configure Settings          | WebUI User    |
| UC0607 | Navigate with Keyboard      | WebUI User    |
| UC0608 | View API Errors             | WebUI User    |
| UC0609 | Expand Graph Node           | WebUI User    |
| UC0610 | Search Documents            | WebUI User    |

### Validation Criteria

- [ ] All BR06XX rules have valid file references
- [ ] All UC06XX cases reference BR and FEAT IDs
- [ ] Summary statistics updated
- [ ] Version numbers incremented
