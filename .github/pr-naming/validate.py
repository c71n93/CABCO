#!/usr/bin/env python3
"""Validate issue-linked metadata for a GitHub pull request."""

import argparse
import re
import sys


BRANCH_PATTERN = re.compile(r"^[1-9][0-9]*-[a-z0-9]+(?:-[a-z0-9]+)*$")
TITLE_PATTERN = re.compile(r"^(?:\[#[1-9][0-9]*\])+ [A-Z].*$")


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--branch", required=True, help="pull request source branch")
    parser.add_argument("--title", required=True, help="pull request title")
    parser.add_argument("--body", required=True, help="pull request body")
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

    if not TITLE_PATTERN.fullmatch(arguments.title):
        print(f"Invalid PR title: {arguments.title}", file=sys.stderr)
        print(
            "Expected format: [#123][#456] Summary starting with A-Z",
            file=sys.stderr,
        )
        return 1

    branch_issue = arguments.branch.split("-", 1)[0]
    title_issue = re.match(r"^\[#([1-9][0-9]*)\]", arguments.title).group(1)
    if title_issue != branch_issue:
        print(
            "Primary issue mismatch: "
            f"branch issue #{branch_issue}, first PR title reference #{title_issue}",
            file=sys.stderr,
        )
        return 1

    closing_pattern = re.compile(
        rf"\b(?:close(?:s|d)?|fix(?:es|ed)?|resolve(?:s|d)?)\s+#{branch_issue}\b",
        re.IGNORECASE,
    )
    if not closing_pattern.search(arguments.body):
        print(
            f"PR body must close primary issue #{branch_issue}; "
            f"for example: Closes #{branch_issue}",
            file=sys.stderr,
        )
        return 1

    print("PR metadata validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
