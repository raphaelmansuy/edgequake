# IMPL-09 Orient — Analysis

## Root Cause

Types were written speculatively during initial SDK creation without cross-referencing the Rust source. The lineage/chunk/provenance types were placed in `health.ts` as a catch-all, with oversimplified shapes.

## Module Organization Improvement

Moved lineage/chunk/provenance types from `health.ts` to dedicated `lineage.ts` — better separation of concerns and discoverability. Legacy aliases maintained in both files for backward compatibility.

## New Capabilities

Added 3 new cost resource methods: `pricing()`, `estimate()`, `workspaceSummary()` — these map to Rust endpoints that were previously unimplemented in the SDK.
