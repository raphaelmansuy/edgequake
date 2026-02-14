# OODA-31: Orient

## Analysis

The Rust SDK had a critical URL bug in provenance.rs and lacked dedicated lineage/settings resources. This put it behind other SDKs (Python, TypeScript, Java, Kotlin, C#, Swift, PHP, Ruby, Go) which all had 7/7 lineage endpoints covered.

## Key Decisions

1. Fix provenance.rs URL from `/api/v1/entities/{}/lineage` to `/api/v1/lineage/entities/{}`
2. Create dedicated `lineage.rs` resource with 4 methods (entity_lineage, document_lineage, document_full_lineage, export_lineage)
3. Create `settings.rs` resource with 2 methods (provider_status, list_providers)
4. Add `get_raw()` method to client for raw byte responses (needed by export_lineage)
5. Fix existing test path regex + add 10 new wiremock tests
