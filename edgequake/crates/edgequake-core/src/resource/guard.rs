//! Resource admission guard — SPEC-006 Layer C (SRP).

use super::budget::ResourceBudgetConfig;

/// Graph operations subject to admission control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphOperation {
    ListEntities,
    ListRelationships,
    DeleteDocument,
    LineageQuery,
    GraphPopularNodes,
    CommunityDetection,
}

impl GraphOperation {
    /// Whether this operation historically required a full-graph scan.
    pub fn requires_full_scan(self) -> bool {
        matches!(self, Self::CommunityDetection)
    }
}

/// Admission decision from pre-flight resource check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionDecision {
    Allow,
    RejectGraphTooLarge { node_count: usize, threshold: usize },
}

/// Pre-flight resource guard (DIP: handlers depend on this, not storage internals).
#[derive(Debug, Clone)]
pub struct ResourceGuard {
    budget: ResourceBudgetConfig,
}

impl ResourceGuard {
    pub fn new(budget: ResourceBudgetConfig) -> Self {
        Self { budget }
    }

    pub fn from_env() -> Self {
        Self::new(ResourceBudgetConfig::from_env())
    }

    pub fn budget(&self) -> &ResourceBudgetConfig {
        &self.budget
    }

    /// SPEC-006: NFR-006-003 — reject before expensive graph work.
    pub fn admit_graph_operation(
        &self,
        operation: GraphOperation,
        node_count: usize,
    ) -> AdmissionDecision {
        if operation.requires_full_scan() && node_count > self.budget.graph_scan_threshold_nodes {
            return AdmissionDecision::RejectGraphTooLarge {
                node_count,
                threshold: self.budget.graph_scan_threshold_nodes,
            };
        }
        AdmissionDecision::Allow
    }
}

impl Default for ResourceGuard {
    fn default() -> Self {
        Self::new(ResourceBudgetConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_community_detection_when_graph_exceeds_threshold() {
        let guard = ResourceGuard::new(ResourceBudgetConfig {
            graph_scan_threshold_nodes: 10_000,
            ..Default::default()
        });
        let decision = guard.admit_graph_operation(GraphOperation::CommunityDetection, 50_000);
        assert_eq!(
            decision,
            AdmissionDecision::RejectGraphTooLarge {
                node_count: 50_000,
                threshold: 10_000
            }
        );
    }

    #[test]
    fn allows_bounded_delete_on_large_graph() {
        let guard = ResourceGuard::default();
        let decision = guard.admit_graph_operation(GraphOperation::DeleteDocument, 200_000);
        assert_eq!(decision, AdmissionDecision::Allow);
    }

    #[test]
    fn allows_list_entities_above_threshold_when_pushdown() {
        let guard = ResourceGuard::default();
        let decision = guard.admit_graph_operation(GraphOperation::ListEntities, 200_000);
        assert_eq!(decision, AdmissionDecision::Allow);
    }
}
