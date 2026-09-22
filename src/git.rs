use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use thiserror::Error;

use crate::{Change, ChangeType, CollectedComparison, CollectedSnapshot};

#[derive(Debug, Error)]
pub enum GitError {
    #[error("cannot run Git in {directory}: {message}")]
    Command { directory: String, message: String },
    #[error("Git operation failed for {input:?}: {message}")]
    Operation { input: String, message: String },
    #[error("Git returned a non-UTF-8 repository path while reading {input:?}")]
    NonUtf8Path { input: String },
    #[error("Git returned an unsupported change type {status:?} for {path:?}")]
    UnsupportedChange { status: String, path: String },
    #[error("Git returned malformed output while reading {input:?}")]
    MalformedOutput { input: String },
}

fn run(directory: &Path, arguments: &[&str], input: &str) -> Result<Output, GitError> {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()
        .map_err(|error| GitError::Command {
            directory: directory.display().to_string(),
            message: error.to_string(),
        })?;
    if !output.status.success() {
        return Err(GitError::Operation {
            input: input.into(),
            message: String::from_utf8_lossy(&output.stderr).trim().into(),
        });
    }
    Ok(output)
}

fn text(output: Output, input: &str) -> Result<String, GitError> {
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| GitError::NonUtf8Path {
            input: input.into(),
        })
}

fn resolve_commit(root: &Path, reference: &str) -> Result<String, GitError> {
    text(
        run(
            root,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{reference}^{{commit}}"),
            ],
            reference,
        )?,
        reference,
    )
}

fn collect_snapshot(root: &Path, commit: String) -> Result<CollectedSnapshot, GitError> {
    let output = run(
        root,
        &["ls-tree", "-r", "--name-only", "-z", &commit],
        &commit,
    )?;
    let files = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| {
            std::str::from_utf8(path)
                .map(str::to_owned)
                .map_err(|_| GitError::NonUtf8Path {
                    input: commit.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let declaration = if files.iter().any(|path| path == "cabco.yaml") {
        Some(run(root, &["show", &format!("{commit}:cabco.yaml")], &commit)?.stdout)
    } else {
        None
    };
    Ok(CollectedSnapshot {
        commit,
        declaration,
        files,
    })
}

fn collect_changes(root: &Path, base: &str, head: &str) -> Result<Vec<Change>, GitError> {
    let input = format!("{base}..{head}");
    let output = run(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--no-renames",
            "--name-status",
            "-z",
            base,
            head,
            "--",
        ],
        &input,
    )?;
    let fields: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect();
    if fields.len() % 2 != 0 {
        return Err(GitError::MalformedOutput { input });
    }
    fields
        .chunks_exact(2)
        .map(|pair| {
            let status = std::str::from_utf8(pair[0]).map_err(|_| GitError::MalformedOutput {
                input: input.clone(),
            })?;
            let path = std::str::from_utf8(pair[1])
                .map_err(|_| GitError::NonUtf8Path {
                    input: input.clone(),
                })?
                .to_owned();
            let change_type = match status {
                "A" => ChangeType::New,
                "D" => ChangeType::Deleted,
                "M" | "T" => ChangeType::Modified,
                _ => {
                    return Err(GitError::UnsupportedChange {
                        status: status.into(),
                        path,
                    });
                }
            };
            Ok(Change { path, change_type })
        })
        .collect()
}

pub fn collect(
    directory: impl AsRef<Path>,
    base_reference: &str,
    head_reference: &str,
) -> Result<CollectedComparison, GitError> {
    let directory = directory.as_ref();
    let root = PathBuf::from(text(
        run(directory, &["rev-parse", "--show-toplevel"], "repository")?,
        "repository",
    )?);
    let base = resolve_commit(&root, base_reference)?;
    let head = resolve_commit(&root, head_reference)?;
    let changes = collect_changes(&root, &base, &head)?;
    Ok(CollectedComparison {
        before: collect_snapshot(&root, base)?,
        after: collect_snapshot(&root, head)?,
        changes,
    })
}
