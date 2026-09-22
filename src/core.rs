use std::fmt;

use serde::Deserialize;
use thiserror::Error;

use crate::declaration::{DeclarationError, resolve};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Ownership {
    Ai,
    Human,
}

impl fmt::Display for Ownership {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Ai => "ai",
            Self::Human => "human",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeType {
    New,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub path: String,
    pub change_type: ChangeType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedSnapshot {
    pub commit: String,
    pub declaration: Option<Vec<u8>>,
    pub files: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedComparison {
    pub before: CollectedSnapshot,
    pub after: CollectedSnapshot,
    pub changes: Vec<Change>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Autonomous,
    Review,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotSide {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasonKind {
    HumanOwned,
    InterfaceBoundary,
    Unassigned,
    NewFile,
    DeclarationChanged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reason {
    pub kind: ReasonKind,
    pub snapshot: Option<SnapshotSide>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathDetails {
    pub component: Option<String>,
    pub ownership: Option<Ownership>,
    pub interface_boundary: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathClassification {
    pub path: String,
    pub change_type: ChangeType,
    pub status: Status,
    pub reasons: Vec<Reason>,
    pub before: Option<PathDetails>,
    pub after: Option<PathDetails>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    pub base_commit: String,
    pub head_commit: String,
    pub paths: Vec<PathClassification>,
}

impl Classification {
    pub fn path(&self, path: &str) -> Option<&PathClassification> {
        self.paths
            .iter()
            .find(|classification| classification.path == path)
    }

    pub fn requires_review(&self) -> bool {
        self.paths.iter().any(|path| path.status == Status::Review)
    }
}

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error(transparent)]
    Declaration(#[from] DeclarationError),
}

fn snapshot_reasons(details: Option<&PathDetails>, side: SnapshotSide) -> Vec<Reason> {
    let details = details.expect("details exist for an applicable snapshot side");
    let mut reasons = Vec::new();
    if details.component.is_none() {
        reasons.push(Reason {
            kind: ReasonKind::Unassigned,
            snapshot: Some(side),
        });
    }
    if details.ownership == Some(Ownership::Human) {
        reasons.push(Reason {
            kind: ReasonKind::HumanOwned,
            snapshot: Some(side),
        });
    }
    if details.interface_boundary {
        reasons.push(Reason {
            kind: ReasonKind::InterfaceBoundary,
            snapshot: Some(side),
        });
    }
    reasons
}

pub fn evaluate(input: CollectedComparison) -> Result<Classification, EvaluationError> {
    let before_declaration = resolve(&input.before)?;
    let after_declaration = resolve(&input.after)?;
    let mut paths = Vec::with_capacity(input.changes.len());

    for change in input.changes {
        let before = (change.change_type != ChangeType::New)
            .then(|| before_declaration.details(&change.path));
        let after = (change.change_type != ChangeType::Deleted)
            .then(|| after_declaration.details(&change.path));
        let mut reasons = Vec::new();
        if change.change_type == ChangeType::New {
            reasons.push(Reason {
                kind: ReasonKind::NewFile,
                snapshot: None,
            });
        }
        if change.change_type != ChangeType::New {
            reasons.extend(snapshot_reasons(before.as_ref(), SnapshotSide::Before));
        }
        if change.change_type != ChangeType::Deleted {
            reasons.extend(snapshot_reasons(after.as_ref(), SnapshotSide::After));
        }
        if change.path == "cabco.yaml" {
            reasons.push(Reason {
                kind: ReasonKind::DeclarationChanged,
                snapshot: None,
            });
        }
        paths.push(PathClassification {
            path: change.path,
            change_type: change.change_type,
            status: if reasons.is_empty() {
                Status::Autonomous
            } else {
                Status::Review
            },
            reasons,
            before,
            after,
        });
    }

    Ok(Classification {
        base_commit: input.before.commit,
        head_commit: input.after.commit,
        paths,
    })
}
