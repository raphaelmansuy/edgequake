//! Deterministic F0 labels from the SPEC-149 E2E contract.

pub const TENANT_A: &str = "tenant-A";
pub const WORKSPACE_A1: &str = "workspace-A1";
pub const WORKSPACE_A2: &str = "workspace-A2";
pub const TENANT_B: &str = "tenant-B";
pub const WORKSPACE_B1: &str = "workspace-B1";

pub const DOCUMENT_A1: &str = "D-A1";
pub const DOCUMENT_A2: &str = "D-A2";
pub const DOCUMENT_A3: &str = "D-A3";
pub const DOCUMENT_B1: &str = "D-B1";
pub const DOCUMENT_A_WORKSPACE_2: &str = "D-AW2";
pub const GRAPH_CYCLE: &str = "G-cycle";

pub const ALPHA_ONLY: &str = "ALPHA_ONLY";
pub const SHARED_ONLY: &str = "SHARED_ONLY";
pub const PENDING_SECRET: &str = "PENDING_SECRET";
pub const BETA_SECRET: &str = "BETA_SECRET";
pub const WORKSPACE_SECRET: &str = "WORKSPACE_SECRET";

pub const EXACT_QUERY: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
pub const EXACT_V1: [f32; 4] = [1.0, 0.0, 0.0, 0.0];
pub const EXACT_V2: [f32; 4] = [0.8, 0.6, 0.0, 0.0];
pub const EXACT_V3: [f32; 4] = [0.0, 1.0, 0.0, 0.0];
pub const EXACT_V4: [f32; 4] = [-1.0, 0.0, 0.0, 0.0];
pub const EXACT_COSINE_TOLERANCE: f32 = 1.0e-5;
