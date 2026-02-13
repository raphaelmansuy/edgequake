# OODA-16: Orient — TypeScript SDK Lineage Tests

## Strategy
- Create dedicated `tests/unit/lineage.test.ts` file for lineage-specific tests
- Use existing `createMockTransport` helper for mock transport setup
- Target 40+ new tests covering all lineage/metadata types
- Focus on field-level validation, not just endpoint connectivity
