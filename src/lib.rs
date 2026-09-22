mod core;
mod declaration;
pub mod git;

pub use core::{
    Change, ChangeType, Classification, CollectedComparison, CollectedSnapshot, Ownership,
    PathClassification, PathDetails, Reason, ReasonKind, SnapshotSide, Status, evaluate,
};
