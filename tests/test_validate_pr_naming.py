import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Optional


ROOT = Path(__file__).resolve().parents[1]
VALIDATOR = ROOT / "scripts" / "validate_pr_naming.py"


class GitRepository:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.git("init", "--initial-branch=master")
        self.git("config", "user.name", "Test User")
        self.git("config", "user.email", "test@example.com")

    def git(self, *args: str) -> str:
        result = subprocess.run(
            ["git", *args],
            cwd=self.path,
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout.strip()

    def commit(self, subject: str, body: Optional[str] = None) -> str:
        marker = self.path / "history.txt"
        previous = marker.read_text() if marker.exists() else ""
        marker.write_text(previous + subject + "\n")
        self.git("add", "history.txt")
        args = ["commit", "-m", subject]
        if body is not None:
            args.extend(["-m", body])
        self.git(*args)
        return self.git("rev-parse", "HEAD")


class ValidatePrNamingTest(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.repository = GitRepository(Path(self.temporary_directory.name))
        self.base = self.repository.commit("Existing base commit")

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def validate(
        self, branch: str, head: str, base: Optional[str] = None
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(VALIDATOR),
                "--branch",
                branch,
                "--base",
                base or self.base,
                "--head",
                head,
            ],
            cwd=self.repository.path,
            capture_output=True,
            text=True,
        )

    def test_valid_branch_and_single_reference_commit_pass(self) -> None:
        head = self.repository.commit("[#10] Enforce naming")

        result = self.validate("10-enforce-pr-naming-v2", head)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Naming validation passed", result.stdout)

    def test_invalid_branch_names_fail_with_expected_format(self) -> None:
        head = self.repository.commit("[#10] Enforce naming")
        invalid_branches = (
            "10",
            "0-description",
            "01-description",
            "10-Uppercase",
            "10-two_words",
            "10-two--words",
            "10-description-",
        )

        for branch in invalid_branches:
            with self.subTest(branch=branch):
                result = self.validate(branch, head)

                self.assertEqual(result.returncode, 1)
                self.assertIn(f"Invalid branch name: {branch}", result.stderr)
                self.assertIn("Expected format: 123-lowercase-kebab-case", result.stderr)

    def test_invalid_commit_subjects_fail_with_commit_diagnostics(self) -> None:
        invalid_subjects = (
            "[10] Missing hash",
            "Missing reference",
            "[#0] Zero issue",
            "[#01] Leading zero",
            "[#1] [#2] Spaced references",
            "[#1]  Extra separator space",
            "[#1] lowercase summary",
        )
        commits = []
        for subject in invalid_subjects:
            commits.append((self.repository.commit(subject), subject))

        result = self.validate("10-enforce-pr-naming", commits[-1][0])

        self.assertEqual(result.returncode, 1)
        self.assertIn(
            "Expected format: [#123][#456] Summary starting with A-Z",
            result.stderr,
        )
        for commit, subject in commits:
            with self.subTest(subject=subject):
                self.assertIn(commit[:12], result.stderr)
                self.assertIn(subject, result.stderr)

    def test_multiple_different_references_and_commit_body_pass(self) -> None:
        head = self.repository.commit(
            "[#1][#2][#300] Fix booking validation",
            body="body text has no formatting restrictions\n[#00] and may look invalid",
        )

        result = self.validate("10-enforce-pr-naming", head)

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_invalid_intermediate_commit_fails_when_latest_commit_is_valid(self) -> None:
        invalid = self.repository.commit("No issue reference")
        head = self.repository.commit("[#10] Finish naming validator")

        result = self.validate("10-enforce-pr-naming", head)

        self.assertEqual(result.returncode, 1)
        self.assertIn(invalid[:12], result.stderr)
        self.assertIn("No issue reference", result.stderr)
        self.assertNotIn(head[:12], result.stderr)

    def test_base_history_and_generated_merge_commit_are_not_checked(self) -> None:
        pr_head = self.repository.commit("[#10] Add naming validator")
        self.repository.git("switch", "-c", "generated-merge", self.base)
        self.repository.git("merge", "--no-ff", pr_head, "-m", "Merge pull request #10")

        result = self.validate("10-enforce-pr-naming", pr_head)

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_locally_squashed_current_history_passes(self) -> None:
        self.repository.commit("Invalid work in progress")
        self.repository.git("reset", "--soft", self.base)
        head = self.repository.commit("[#10] Squash naming changes")

        result = self.validate("10-enforce-pr-naming", head)

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_unknown_revision_returns_command_error(self) -> None:
        head = self.repository.commit("[#10] Enforce naming")

        result = self.validate("10-enforce-pr-naming", head, base="unknown-base")

        self.assertEqual(result.returncode, 2)
        self.assertIn("Unable to read pull request history:", result.stderr)


if __name__ == "__main__":
    unittest.main()
