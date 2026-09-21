import subprocess
import sys
import unittest
from pathlib import Path


VALIDATOR = Path(__file__).with_name("validate.py")


class ValidatePrMetadataTest(unittest.TestCase):
    def validate(
        self, branch: str, title: str, body: str
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(VALIDATOR),
                "--branch",
                branch,
                "--title",
                title,
                "--body",
                body,
            ],
            capture_output=True,
            text=True,
        )

    def test_valid_primary_issue_metadata_passes(self) -> None:
        result = self.validate(
            "123-fix-booking",
            "[#123] Fix booking validation",
            "Closes #123",
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PR metadata validation passed", result.stdout)

    def test_additional_title_references_and_supported_closing_keywords_pass(self) -> None:
        closing_keywords = (
            "Close",
            "Closes",
            "Closed",
            "Fix",
            "Fixes",
            "Fixed",
            "Resolve",
            "Resolves",
            "Resolved",
        )

        for keyword in closing_keywords:
            with self.subTest(keyword=keyword):
                result = self.validate(
                    "123-fix-booking-v2",
                    "[#123][#456] Fix booking validation",
                    f"{keyword} #123.\n\nAlso closes #456.",
                )

                self.assertEqual(result.returncode, 0, result.stderr)

    def test_invalid_branch_names_fail_with_expected_format(self) -> None:
        invalid_branches = (
            "123",
            "0-description",
            "012-description",
            "123-Uppercase",
            "123-two_words",
            "123-two--words",
            "123-description-",
        )

        for branch in invalid_branches:
            with self.subTest(branch=branch):
                result = self.validate(
                    branch,
                    "[#123] Fix booking validation",
                    "Closes #123",
                )

                self.assertEqual(result.returncode, 1)
                self.assertIn(f"Invalid branch name: {branch}", result.stderr)
                self.assertIn("Expected format: 123-lowercase-kebab-case", result.stderr)

    def test_invalid_pr_title_format_fails_with_expected_format(self) -> None:
        invalid_titles = (
            "[123] Missing hash",
            "Missing reference",
            "[#0] Zero issue",
            "[#01] Leading zero",
            "[#123] [#456] Spaced references",
            "[#123]  Extra separator space",
            "[#123] lowercase summary",
        )

        for title in invalid_titles:
            with self.subTest(title=title):
                result = self.validate("123-fix-booking", title, "Closes #123")

                self.assertEqual(result.returncode, 1)
                self.assertIn(f"Invalid PR title: {title}", result.stderr)
                self.assertIn(
                    "Expected format: [#123][#456] Summary starting with A-Z",
                    result.stderr,
                )

    def test_first_title_reference_must_match_branch_issue(self) -> None:
        result = self.validate(
            "123-fix-booking",
            "[#456][#123] Fix booking validation",
            "Closes #123",
        )

        self.assertEqual(result.returncode, 1)
        self.assertIn("Primary issue mismatch", result.stderr)
        self.assertIn("branch issue #123", result.stderr)
        self.assertIn("first PR title reference #456", result.stderr)

    def test_body_must_close_branch_issue(self) -> None:
        invalid_bodies = (
            "No issue relationship",
            "Closes #456",
            "Refs #123",
        )

        for body in invalid_bodies:
            with self.subTest(body=body):
                result = self.validate(
                    "123-fix-booking",
                    "[#123] Fix booking validation",
                    body,
                )

                self.assertEqual(result.returncode, 1)
                self.assertIn("PR body must close primary issue #123", result.stderr)
                self.assertIn("Closes #123", result.stderr)


if __name__ == "__main__":
    unittest.main()
