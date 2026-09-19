#!/usr/bin/env python3
"""Validate naming for a GitHub pull request branch and its introduced commits."""

import argparse
import re
import subprocess
import sys


BRANCH_PATTERN = re.compile(r"^[1-9][0-9]*-[a-z0-9]+(?:-[a-z0-9]+)*$")
SUBJECT_PATTERN = re.compile(r"^(?:\[#[1-9][0-9]*\])+ [A-Z].*$")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--branch", required=True, help="pull request source branch")
    parser.add_argument("--base", required=True, help="pull request base commit")
    parser.add_argument("--head", required=True, help="pull request head commit")
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    if not BRANCH_PATTERN.fullmatch(arguments.branch):
        print(f"Invalid branch name: {arguments.branch}", file=sys.stderr)
        print(
            "Expected format: 123-lowercase-kebab-case",
            file=sys.stderr,
        )
        return 1

    try:
        history = subprocess.run(
            [
                "git",
                "log",
                "--format=%H%x09%s",
                f"{arguments.base}..{arguments.head}",
            ],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() or "Git did not provide an error message"
        print(f"Unable to read pull request history: {detail}", file=sys.stderr)
        return 2

    invalid_commits = []
    for line in history.splitlines():
        commit, subject = line.split("\t", 1)
        if not SUBJECT_PATTERN.fullmatch(subject):
            invalid_commits.append((commit, subject))

    if invalid_commits:
        for commit, subject in invalid_commits:
            print(f"Invalid commit {commit[:12]}: {subject}", file=sys.stderr)
        print(
            "Expected format: [#123][#456] Summary starting with A-Z",
            file=sys.stderr,
        )
        return 1

    print("Naming validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
