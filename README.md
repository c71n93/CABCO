# CABCO

**Code Autonomy Bounded by Component Ownership**

CABCO is a project-level policy framework for controlling AI autonomy through explicit component ownership while keeping interfaces, contracts, and sensitive modules under human control.

It aims to:
- declare components as AI- or human-owned;
- allow autonomous changes inside AI-owned components;
- require human review when changes cross declared boundaries;
- keep ownership rules explicit, version-controlled, and tool-independent.

CABCO includes an initial Rust CLI and reusable policy library for classifying
committed file changes.

## Usage

Build with Cargo and compare two commit IDs or Git references:

```console
cargo build
cargo run -- check --base <commit> --head <commit>
```

Both references are required. CABCO resolves each to a commit, compares the two
complete snapshots directly with rename detection disabled, and ignores the
working tree and remotes. Exit code `0` means evaluation succeeded with only
`AUTONOMOUS` paths, `1` means at least one path requires `REVIEW`, and `2` means
evaluation failed.

`AUTONOMOUS` only means the declared ownership and interface-boundary rules do
not require human review. It does not prove correctness, unchanged behavior, or
contract satisfaction, and it does not record approval.

## Declarations

Each selected snapshot may contain a repository-root `cabco.yaml`:

```yaml
version: 1
components:
  application:
    ownership: ai
    files:
      - src/**
    interfaces:
      - src/api.rs
```

Version 1 requires every component to declare `ownership` (`ai` or `human`),
`files`, and `interfaces`. An empty `interfaces` list is valid. Unknown fields,
duplicate keys, overlaps between components, and boundary files outside their
declaring component are errors. A missing declaration means that snapshot's
files are unassigned. An invalid declaration in either selected snapshot stops
evaluation, including when the later commit repairs it.

Patterns use the established Rust [`globset`](https://docs.rs/globset/0.4/)
syntax. They are matched against the complete repository-relative path with
case-sensitive forward-slash separators. Literal paths, `*`, `?`, character
classes, brace alternatives, and recursive `**` patterns are supported;
tracked dotfiles are not excluded. Patterns are rooted by matching the full
repository-relative path, so `src/**` does not match `other/src/file`. CABCO
does not support exclusions or rule-order precedence. Unmatched patterns and
multiple matching patterns in the same component are valid.

Format version 1 identifies a compatible declaration generation, not a CABCO
release. Compatible documented meaning retains version 1; incompatible
structure or interpretation requires a future format version. This prototype
does not migrate declarations.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the issue-linked branch and commit
naming rules applied to pull requests.
