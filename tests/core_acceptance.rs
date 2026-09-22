use cabco::{
    Change, ChangeType, CollectedComparison, CollectedSnapshot, ReasonKind, SnapshotSide, Status,
    evaluate,
};

fn snapshot(commit: &str, declaration: Option<&str>, files: &[&str]) -> CollectedSnapshot {
    CollectedSnapshot {
        commit: commit.into(),
        declaration: declaration.map(str::as_bytes).map(Vec::from),
        files: files.iter().map(|path| (*path).into()).collect(),
    }
}

fn change(path: &str, change_type: ChangeType) -> Change {
    Change {
        path: path.into(),
        change_type,
    }
}

#[test]
fn evaluation_unions_review_requirements_and_returns_structured_details() {
    let before = snapshot(
        "before-id",
        Some(
            r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**]
    interfaces: [src/api.rs]
  manual:
    ownership: human
    files: [manual/**]
    interfaces: []
"#,
        ),
        &[
            "src/internal.rs",
            "src/api.rs",
            "manual/run.rs",
            "loose.txt",
            "cabco.yaml",
        ],
    );
    let after = snapshot(
        "after-id",
        Some(
            r#"version: 1
components:
  app:
    ownership: human
    files: [src/internal.rs, src/new.rs]
    interfaces: [src/new.rs]
"#,
        ),
        &["src/internal.rs", "src/new.rs", "cabco.yaml"],
    );
    let result = evaluate(CollectedComparison {
        before,
        after,
        changes: vec![
            change("src/internal.rs", ChangeType::Modified),
            change("src/api.rs", ChangeType::Deleted),
            change("manual/run.rs", ChangeType::Deleted),
            change("loose.txt", ChangeType::Deleted),
            change("src/new.rs", ChangeType::New),
            change("cabco.yaml", ChangeType::Modified),
        ],
    })
    .unwrap();

    assert_eq!(result.base_commit, "before-id");
    assert_eq!(result.head_commit, "after-id");
    let internal = result.path("src/internal.rs").unwrap();
    assert_eq!(internal.status, Status::Review);
    assert_eq!(
        internal
            .before
            .as_ref()
            .unwrap()
            .ownership
            .unwrap()
            .to_string(),
        "ai"
    );
    assert_eq!(
        internal
            .after
            .as_ref()
            .unwrap()
            .ownership
            .unwrap()
            .to_string(),
        "human"
    );
    assert_eq!(internal.reasons[0].kind, ReasonKind::HumanOwned);
    assert_eq!(internal.reasons[0].snapshot, Some(SnapshotSide::After));

    let boundary = result.path("src/api.rs").unwrap();
    assert!(boundary.reasons.iter().any(|reason| {
        reason.kind == ReasonKind::InterfaceBoundary
            && reason.snapshot == Some(SnapshotSide::Before)
    }));
    assert!(
        result
            .path("manual/run.rs")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| {
                reason.kind == ReasonKind::HumanOwned
                    && reason.snapshot == Some(SnapshotSide::Before)
            })
    );
    assert!(
        result
            .path("loose.txt")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| {
                reason.kind == ReasonKind::Unassigned
                    && reason.snapshot == Some(SnapshotSide::Before)
            })
    );

    let new_file = result.path("src/new.rs").unwrap();
    assert_eq!(new_file.reasons.len(), 3);
    assert!(
        new_file
            .reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::NewFile)
    );
    assert!(
        new_file
            .reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::HumanOwned)
    );
    assert!(
        new_file
            .reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::InterfaceBoundary)
    );
    assert!(
        result
            .path("cabco.yaml")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| { reason.kind == ReasonKind::DeclarationChanged })
    );
}

#[test]
fn ai_internal_edits_and_deletions_are_autonomous() {
    let declaration = r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**]
    interfaces: []
"#;
    let result = evaluate(CollectedComparison {
        before: snapshot("one", Some(declaration), &["src/edit.rs", "src/delete.rs"]),
        after: snapshot("two", Some(declaration), &["src/edit.rs"]),
        changes: vec![
            change("src/edit.rs", ChangeType::Modified),
            change("src/delete.rs", ChangeType::Deleted),
        ],
    })
    .unwrap();

    assert!(
        result
            .paths
            .iter()
            .all(|path| path.status == Status::Autonomous)
    );
    assert!(result.paths.iter().all(|path| path.reasons.is_empty()));
}

#[test]
fn declarations_validate_globs_and_every_snapshot_file() {
    let valid = r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**/*.rs, src/main.rs, README.md, .github/**, unmatched/**]
    interfaces: []
"#;
    let result = evaluate(CollectedComparison {
        before: snapshot(
            "one",
            Some(valid),
            &[
                "src/main.rs",
                "src/pkg/lib.rs",
                "README.md",
                ".github/workflows/ci.yml",
                "readme.md",
            ],
        ),
        after: snapshot(
            "two",
            Some(valid),
            &[
                "src/main.rs",
                "src/pkg/lib.rs",
                "README.md",
                ".github/workflows/ci.yml",
                "readme.md",
            ],
        ),
        changes: vec![
            change("src/main.rs", ChangeType::Modified),
            change("src/pkg/lib.rs", ChangeType::Modified),
            change("README.md", ChangeType::Modified),
            change(".github/workflows/ci.yml", ChangeType::Modified),
            change("readme.md", ChangeType::Modified),
        ],
    })
    .unwrap();
    for path in [
        "src/main.rs",
        "src/pkg/lib.rs",
        "README.md",
        ".github/workflows/ci.yml",
    ] {
        assert_eq!(
            result.path(path).unwrap().status,
            Status::Autonomous,
            "{path}"
        );
    }
    assert_eq!(
        result.path("readme.md").unwrap().status,
        Status::Review,
        "matching is case-sensitive"
    );

    let overlap = r#"version: 1
components:
  first:
    ownership: ai
    files: [src/**, src/main.rs]
    interfaces: []
  second:
    ownership: ai
    files: [src/main.rs]
    interfaces: []
"#;
    let error = evaluate(CollectedComparison {
        before: snapshot("one", Some(overlap), &["src/main.rs"]),
        after: snapshot("two", None, &[]),
        changes: vec![],
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("src/main.rs") && error.contains("first") && error.contains("second"));

    let external_boundary = r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**]
    interfaces: [docs/api.md]
"#;
    let error = evaluate(CollectedComparison {
        before: snapshot(
            "one",
            Some(external_boundary),
            &["src/main.rs", "docs/api.md"],
        ),
        after: snapshot("two", None, &[]),
        changes: vec![],
    })
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("docs/api.md") && error.contains("app") && error.contains("does not belong")
    );
}

#[test]
fn invalid_declarations_fail_even_when_repaired_or_no_paths_changed() {
    let invalid_documents = [
        ("version: [", "invalid YAML"),
        ("version: 2\ncomponents: {}", "unsupported version"),
        ("version: true\ncomponents: {}", "version"),
        ("version: 1\ncomponents: {}\nextra: true", "unknown field"),
        ("version: 1\ncomponents: []", "components"),
        (
            "version: 1\ncomponents:\n  app:\n    ownership: robot\n    files: []\n    interfaces: []",
            "ownership",
        ),
        (
            "version: 1\ncomponents:\n  app:\n    ownership: ai\n    files: src/**\n    interfaces: []",
            "files",
        ),
        (
            "version: 1\ncomponents:\n  app:\n    ownership: ai\n    files: []",
            "interfaces",
        ),
        (
            "version: 1\ncomponents:\n  app:\n    ownership: ai\n    files: []\n    files: []\n    interfaces: []",
            "duplicate",
        ),
        ("version: 1\ncomponents: {}\nversion: 1", "duplicate"),
        (
            "version: 1\ncomponents:\n  app: { ownership: ai, files: [], interfaces: [] }\n  app: { ownership: ai, files: [], interfaces: [] }",
            "duplicate",
        ),
        ("components: {}", "version"),
    ];
    for (document, expected) in invalid_documents {
        let error = evaluate(CollectedComparison {
            before: snapshot("bad-before", Some(document), &["cabco.yaml"]),
            after: snapshot(
                "repaired",
                Some("version: 1\ncomponents: {}"),
                &["cabco.yaml"],
            ),
            changes: vec![],
        })
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("bad-before") && error.contains(expected),
            "{error}"
        );
    }
}

#[test]
fn missing_declarations_and_membership_changes_require_review_on_the_applicable_side() {
    let assigned = r#"version: 1
components:
  app:
    ownership: ai
    files: [src/assigned.rs]
    interfaces: []
"#;
    let result = evaluate(CollectedComparison {
        before: snapshot(
            "before",
            Some(assigned),
            &["src/assigned.rs", "src/becomes-assigned.rs", "loose.txt"],
        ),
        after: snapshot(
            "after",
            None,
            &[
                "src/assigned.rs",
                "src/becomes-assigned.rs",
                "loose.txt",
                "new.txt",
            ],
        ),
        changes: vec![
            change("src/assigned.rs", ChangeType::Modified),
            change("src/becomes-assigned.rs", ChangeType::Modified),
            change("loose.txt", ChangeType::Modified),
            change("new.txt", ChangeType::New),
        ],
    })
    .unwrap();

    assert!(
        result
            .path("src/assigned.rs")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| {
                reason.kind == ReasonKind::Unassigned
                    && reason.snapshot == Some(SnapshotSide::After)
            })
    );
    assert!(result.path("src/assigned.rs").unwrap().after.is_some());
    assert!(
        result
            .path("src/assigned.rs")
            .unwrap()
            .after
            .as_ref()
            .unwrap()
            .component
            .is_none()
    );
    assert!(
        result
            .path("src/becomes-assigned.rs")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| {
                reason.kind == ReasonKind::Unassigned
                    && reason.snapshot == Some(SnapshotSide::Before)
            })
    );
    assert_eq!(
        result
            .path("loose.txt")
            .unwrap()
            .reasons
            .iter()
            .filter(|reason| reason.kind == ReasonKind::Unassigned)
            .count(),
        2
    );
    let new_file = result.path("new.txt").unwrap();
    assert!(
        new_file
            .reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::NewFile)
    );
    assert!(new_file.reasons.iter().any(|reason| {
        reason.kind == ReasonKind::Unassigned && reason.snapshot == Some(SnapshotSide::After)
    }));
}

#[test]
fn removed_restrictions_and_declaration_changes_cannot_remove_review() {
    let before = r#"version: 1
components:
  app:
    ownership: human
    files: [src/**]
    interfaces: [src/old-api.rs]
"#;
    let after = r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**]
    interfaces: [src/new-api.rs]
"#;
    let result = evaluate(CollectedComparison {
        before: snapshot(
            "before",
            Some(before),
            &["cabco.yaml", "src/old-api.rs", "src/new-api.rs"],
        ),
        after: snapshot(
            "after",
            Some(after),
            &["cabco.yaml", "src/old-api.rs", "src/new-api.rs"],
        ),
        changes: vec![
            change("cabco.yaml", ChangeType::Modified),
            change("src/old-api.rs", ChangeType::Modified),
            change("src/new-api.rs", ChangeType::Modified),
        ],
    })
    .unwrap();

    let old = result.path("src/old-api.rs").unwrap();
    assert!(
        old.reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::HumanOwned)
    );
    assert!(
        old.reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::InterfaceBoundary)
    );
    let new = result.path("src/new-api.rs").unwrap();
    assert!(
        new.reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::HumanOwned)
    );
    assert!(
        new.reasons
            .iter()
            .any(|reason| reason.kind == ReasonKind::InterfaceBoundary)
    );
    assert_eq!(
        result.paths.len(),
        3,
        "unchanged files are not added to the report"
    );
}

#[test]
fn declaration_creation_and_deletion_are_review_reasons() {
    for (change_type, before, after) in [
        (
            ChangeType::New,
            snapshot("before", None, &[]),
            snapshot("after", Some("version: 1\ncomponents: {}"), &["cabco.yaml"]),
        ),
        (
            ChangeType::Deleted,
            snapshot(
                "before",
                Some("version: 1\ncomponents: {}"),
                &["cabco.yaml"],
            ),
            snapshot("after", None, &[]),
        ),
    ] {
        let result = evaluate(CollectedComparison {
            before,
            after,
            changes: vec![change("cabco.yaml", change_type)],
        })
        .unwrap();
        assert!(
            result.paths[0]
                .reasons
                .iter()
                .any(|reason| { reason.kind == ReasonKind::DeclarationChanged })
        );
    }
}
