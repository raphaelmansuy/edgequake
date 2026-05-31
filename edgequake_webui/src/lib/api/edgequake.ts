/**
 * @module edgequake-api
 * @description Barrel re-export for EdgeQuake REST API client (SPEC-017 UI-DRY-001).
 *
 * Domain implementations live in `./edgequake/` submodules.
 * This file preserves the `@/lib/api/edgequake` import path for all consumers.
 */

export * from "./edgequake/health";
export * from "./edgequake/auth";
export * from "./edgequake/workspaces";
export * from "./edgequake/documents";
export * from "./edgequake/query";
export * from "./edgequake/graph";
export * from "./edgequake/pipeline";
export * from "./edgequake/lineage";
export * from "./edgequake/cost";
export * from "./edgequake/injection";

export { edgequakeApi, default } from "./edgequake/default-export";

// Re-export conversations/folders/query-keys (legacy path)
export * from "./conversations";
export * from "./folders";
export * from "./query-keys";
