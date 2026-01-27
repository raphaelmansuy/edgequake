You are a senior UX/UI designer. Review every screen and major container of this web application end-to-end.

Use #playwright to navigate the product like a real user and capture evidence (observations + what screen/state you were on). For each screen, evaluate:

Information architecture & layout
Visual hierarchy (primary vs secondary actions, scanning order)
Space optimization (density, alignment, spacing consistency)
Typography (scale, line-height, contrast, weights, readability)
Navigation & discoverability
Accessibility basics (contrast, focus states, keyboard nav cues)
Product requirements to prioritize
Add/validate collapsible left and right panels
Recommend default states, collapse behavior, resize behavior, and persistence.
Ensure a strong visual hierarchy
Optimize space without making the UI feel cramped
Improve typography system and consistency
Deliverable format
For each screen/container, produce:

What I reviewed (route/page name + key UI regions)
Issues (bulleted, grouped by severity: Critical / Major / Minor)
Recommendations (specific changes, not generic advice)
Rationale (why it improves UX/UI)
Acceptance criteria (clear “done when…” checks)
Finish with:

A prioritized roadmap (Quick wins / Next / Later)
A proposed layout + typography system (tokens: type scale, spacing scale, panel widths)
Any design patterns you recommend standardizing (panels, tables, forms, empty states)

Write your audit in markdown format with screenshots embedded where relevant.

in audit_ui/ 

- A file by screen container, e.g. audit_ui/dashboard.md
- Use ASCII diagrams where helpful to represent the layout after your recommendations
- A summary file audit_ui/summary.md with the roadmap, design system tokens, and patterns
- Ensure to reference specific files and components from the codebase where relevant
- Ensure to cross-reference any UX improvements documented 