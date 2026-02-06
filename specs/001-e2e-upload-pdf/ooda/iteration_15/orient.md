# OODA-15 Orient

All edge cases should be tested as black-box HTTP tests. Assertions should accept multiple valid response codes (CREATED, BAD_REQUEST, UNPROCESSABLE_ENTITY) since some edge cases are implementation-dependent. The key invariant: never panic, always return structured JSON.
