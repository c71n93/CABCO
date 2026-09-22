mod core;
mod declaration;
pub mod git;

pub use core::{
    Change, ChangeType, Classification, CollectedComparison, CollectedSnapshot, EvaluationError,
    Ownership, PathClassification, PathDetails, Reason, ReasonKind, SnapshotSide, Status, evaluate,
};
pub use declaration::DeclarationError;
