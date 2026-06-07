//! Shared entity schema configuration for LLM and SOTA extractors (SPEC-017 DRY-008).

use crate::prompts::EntityExtractionSchema;

/// Extractors that carry a workspace entity schema share these builder methods.
pub trait ConfigurableEntitySchema: Sized {
    /// Mutable reference to the embedded schema (implementor holds the field).
    fn entity_schema_field(&mut self) -> &mut EntityExtractionSchema;

    /// Restrict extraction to custom entity types (strict enforcement).
    fn with_entity_types(mut self, types: Vec<String>) -> Self {
        *self.entity_schema_field() = EntityExtractionSchema::with_types(types);
        self
    }

    /// Set full schema (types + strict/permissive mode).
    fn with_entity_schema(mut self, schema: EntityExtractionSchema) -> Self {
        *self.entity_schema_field() = schema;
        self
    }
}
