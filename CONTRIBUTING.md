# Contributing

Every change must be backed by a GitHub issue. Pull requests enforce the branch
and commit naming rules below by syntax; the check does not query GitHub to
confirm that a referenced number exists, is an issue rather than a pull request,
or is open.

## Branch names

Start the branch name with one positive issue number without leading zeroes,
then a hyphen and a lowercase kebab-case description. Description segments may
contain lowercase English letters and digits.

Valid examples:

- `10-enforce-pr-naming`
- `123-fix-booking-v2`

Invalid examples include `10` (no description), `0-fix`, `01-fix`,
`10-Fix-booking`, `10-fix_booking`, `10-fix--booking`, and `10-fix-`.

## Commit subjects

Begin every commit subject with one or more adjacent issue references. Use a
positive issue number without leading zeroes in each reference, include the
hash sign, place no spaces between references, and put exactly one space before
a summary whose first character is an uppercase English letter (`A`–`Z`). A
commit may reference an issue other than the branch's primary issue. Commit
bodies have no additional formatting rules.

Valid examples:

- `[#123] Add booking validation`
- `[#1][#2][#3] Fix booking validation`

Invalid examples include `[123] Missing hash`, `Missing reference`,
`[#0] Zero issue`, `[#01] Leading zero`, `[#1] [#2] Spaced references`,
`[#1]  Extra space`, and `[#1] lowercase summary`.

The naming check runs for pull requests when they are opened, reopened, or
updated. It validates the source branch and all commits introduced by the pull
request. It ignores existing base-branch history and GitHub's generated test
merge commit, and it does not run for direct pushes.

The repository uses rebase merging. Keep meaningful commits or squash them
locally before merging; every commit remaining in the current pull request
history must follow the subject format. Updating, rebasing, or force-pushing the
branch reruns the check against that current history.
