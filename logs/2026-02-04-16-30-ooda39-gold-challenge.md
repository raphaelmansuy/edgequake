# 2026-02-04-16-30 OODA-39 Gold Standard Challenge

## Actions

- Used markitdown MCP to extract one_tool_2512.20957v2.pdf
- Compared output with gold standard to identify synthesized content
- Updated gold standard to remove **Authors:** and **Affiliation:** lines
- Created OODA-39 documentation (observe/orient/decide/act)
- Committed changes to repository

## Decisions

- Gold standard should represent "faithful extraction" not "semantic synthesis"
- markitdown (Microsoft's official tool, 86K⭐) validates our extraction approach
- Affiliations appearing mid-document is PHYSICALLY CORRECT per PDF layout
- F1 score remained at 0.752 because genuine extraction issues remain

## Next Steps

- OODA-40: Address two-column text interleaving (`repositowhich`)
- Fix author name spacing (`Zhaoxi ZhangYitong Duan` -> with spaces)
- Investigate block-level column separation

## Lessons/Insights

- Always verify gold standards against reference tools before assuming extraction is wrong
- First principles: faithful extraction > semantic synthesis for RAG systems
- markitdown MCP is a valuable validation tool for PDF extraction quality
