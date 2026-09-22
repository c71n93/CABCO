use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn write(directory: &Path, path: &str, content: &str) {
    let destination = directory.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(destination, content).unwrap();
}

fn commit(directory: &Path, message: &str) -> String {
    git(directory, &["add", "."]);
    git(directory, &["commit", "-m", message]);
    git(directory, &["rev-parse", "HEAD"])
}

fn cabco(directory: &Path, base: &str, head: &str) -> Output {
    let nested = directory.join("src");
    Command::new(env!("CARGO_BIN_EXE_cabco"))
        .current_dir(if nested.is_dir() {
            nested.as_path()
        } else {
            directory
        })
        .args(["check", "--base", base, "--head", head])
        .output()
        .unwrap()
}

#[test]
fn cli_classifies_selected_commits_with_deterministic_output_and_codes() {
    let repository = tempfile::tempdir().unwrap();
    git(repository.path(), &["init", "-q"]);
    git(
        repository.path(),
        &["config", "user.email", "test@example.com"],
    );
    git(repository.path(), &["config", "user.name", "CABCO Test"]);
    write(
        repository.path(),
        "cabco.yaml",
        r#"version: 1
components:
  app:
    ownership: ai
    files: [src/**]
    interfaces: [src/api.rs]
  docs:
    ownership: human
    files: [docs/**]
    interfaces: []
"#,
    );
    write(repository.path(), "src/internal.rs", "one\n");
    write(repository.path(), "src/api.rs", "one\n");
    write(repository.path(), "src/moved.rs", "move me\n");
    write(repository.path(), "docs/guide.md", "one\n");
    let base = commit(repository.path(), "base");

    write(repository.path(), "src/internal.rs", "two\n");
    write(repository.path(), "src/api.rs", "two\n");
    fs::rename(
        repository.path().join("src/moved.rs"),
        repository.path().join("src/new-name.rs"),
    )
    .unwrap();
    write(repository.path(), "docs/guide.md", "two\n");
    write(repository.path(), "src/new.rs", "new\n");
    let head = commit(repository.path(), "head");

    write(
        repository.path(),
        "cabco.yaml",
        "not: the selected declaration\n",
    );
    write(repository.path(), "untracked.txt", "ignored\n");

    let output = cabco(repository.path(), &base, &head);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "REVIEW (4)\n",
            "docs/guide.md\n",
            "src/api.rs\n",
            "src/new-name.rs\n",
            "src/new.rs\n",
            "AUTONOMOUS (2)\n",
            "src/internal.rs\n",
            "src/moved.rs\n",
        )
    );
}

#[test]
fn cli_validates_identical_snapshots_and_reports_errors_without_a_verdict() {
    let repository = tempfile::tempdir().unwrap();
    git(repository.path(), &["init", "-q"]);
    git(
        repository.path(),
        &["config", "user.email", "test@example.com"],
    );
    git(repository.path(), &["config", "user.name", "CABCO Test"]);
    write(
        repository.path(),
        "cabco.yaml",
        "version: 1\ncomponents: {}\n",
    );
    let valid = commit(repository.path(), "valid");

    let output = cabco(repository.path(), &valid, &valid);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "AUTONOMOUS — no changed files\n"
    );

    write(repository.path(), "cabco.yaml", "version: [\n");
    let invalid = commit(repository.path(), "invalid");
    let output = cabco(repository.path(), &invalid, &invalid);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains(&invalid) && error.contains("invalid YAML"),
        "{error}"
    );
}

#[test]
fn cli_rejects_invalid_commit_references() {
    let repository = tempfile::tempdir().unwrap();
    git(repository.path(), &["init", "-q"]);
    let output = cabco(repository.path(), "missing", "also-missing");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("missing")
    );
}
