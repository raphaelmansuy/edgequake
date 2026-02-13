# OODA-17: Orient — Python SDK Lineage Tests

## Strategy
- Create dedicated `tests/test_lineage.py` for lineage type tests
- Use pydantic model_dump / model_validate for serialization roundtrip tests
- Test all lineage types at field level with comprehensive assertions
- Cover edge cases: empty metadata, zero counts, nested structures
