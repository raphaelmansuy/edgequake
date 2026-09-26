//! Reserved fault-barrier vocabulary for SPEC-149 recovery scenarios.
//!
//! J01 does not expose runtime fault controls. J09+ may add test-build-only IPC
//! behind a dedicated Cargo feature; release builds must remain unreachable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // reserved for J09+ recovery barriers
pub enum FaultBarrier {
    StagedInsertBeforeEventAppend,
    AuthorityCommitBeforeDelivery,
    ProviderApplyBeforeLedgerAck,
    LeaseClaimBeforeProviderCompletion,
    TombstoneBeforeCleanup,
    OneTargetReceipt,
    MigrationPageBeforeCheckpoint,
    BindingGenerationSwitch,
}

pub fn fault_control_available() -> bool {
    false
}
