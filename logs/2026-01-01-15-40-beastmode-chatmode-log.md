# Task logs

- Actions: Investigated failing edge-case tests; improved lopdf startxref recovery; fixed lopdf encryption detection; reran full edgequake-pdf test suite.
- Decisions: Prefer parser resilience (scan near startxref) over weakening tests; treat presence of `/Encrypt` in trailer as encrypted even if encryption dictionary isn’t resolvable in partial load.
- Next steps: Re-run extraction on real_dataset/ccn_2512.21804v1.pdf and continue with table reconstruction + figure/image handling (vision).
- Lessons/insights: Real-world PDFs often have slightly wrong `startxref`; resilient xref discovery prevents brittle failures and improves downstream extraction stability.
