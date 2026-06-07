/**
 * General UI bootstrap — re-exports SPEC-013 helpers for all integration tests.
 * First principle: every UI test needs deterministic tenant/workspace context.
 */
export {
  bootstrapDeterministicUiContext,
  seedTenantStoreOnPage,
  openCreateWorkspaceDialog,
  createTenantWorkspaceViaApi,
  type Spec013BootstrapContext,
} from "./spec013-bootstrap";

export { tenantHeaders } from "./spec013-api";
