-- Migration 041: Document stats columns for relational read model (SPEC-021 P-A1)
--
-- WHY: `documents.entity_count` / `chunk_count` existed (migration 001/003) but were
-- never updated by any production writer. P5-01 wired the relational `documents` table
-- into the documents-list read path as a fallback, which promoted these dead columns
-- to read-path inputs and produced the "Completed / 0 entities" screenshot (file 16).
--
-- This migration adds the missing denormalized stat/cost columns so that
-- `update_document_stats` (new trait method on PdfDocumentStorage) can refresh them
-- after every ingestion. All columns are nullable to stay additive and backward
-- compatible with existing rows.
--
-- @implements SPEC-021 P-A1: Write-path closure for the per-doc read model.

-- Relationship count mirrors the existing chunk_count/entity_count denormalization.
ALTER TABLE documents ADD COLUMN IF NOT EXISTS relationship_count INTEGER DEFAULT 0;

-- Cost / token tracking columns (KV metadata already carries these; relational
-- backfill rows currently return NULL → UI shows "-"). Nullable: legacy rows and
-- mock provider ingestions legitimately have no cost.
ALTER TABLE documents ADD COLUMN IF NOT EXISTS cost_usd      DOUBLE PRECISION;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS input_tokens   BIGINT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS output_tokens  BIGINT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS total_tokens   BIGINT;

-- Error details mirror the KV `error_message` field so partial_failure rows
-- surfaced from the relational backfill carry the same diagnostic text.
ALTER TABLE documents ADD COLUMN IF NOT EXISTS error_message TEXT;

COMMENT ON COLUMN documents.chunk_count        IS 'Denormalized chunk count, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.entity_count       IS 'Denormalized entity count, refreshed by update_document_stats (SPEC-021 P-A1). SSOT for this fact = AGE graph (INV-C).';
COMMENT ON COLUMN documents.relationship_count IS 'Denormalized relationship count, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.cost_usd           IS 'LLM cost in USD for this document, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.input_tokens       IS 'LLM input tokens, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.output_tokens      IS 'LLM output tokens, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.total_tokens       IS 'LLM total tokens, refreshed by update_document_stats (SPEC-021 P-A1).';
COMMENT ON COLUMN documents.error_message      IS 'Terminal/partial failure detail, refreshed by update_document_stats (SPEC-021 P-A1).';
