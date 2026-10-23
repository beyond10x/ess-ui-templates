# Task: Name ESS 0.68.0 in the Taskfile's BILLING_REPO and in AGENTS.md

Part of `story:ess-068-crates`; covers `specification:ess-068-crates` S3.2, S3.3, S5.2, and S3.1
limited to these two files.

## Scope

- `Taskfile.yml` line 11 (`BILLING_REPO`, cited)
- `AGENTS.md` lines 24-25 (§ Rules, cited)

## Work

In `BILLING_REPO`, change `--ref 4579d8049bb5b83749b46059dfc0982de7633485` to
`--ref ae9d3604ec001b3f0f318e8d961748c388082f60`. In `AGENTS.md` § Rules, state the pin as
`ESS 0.68.0 (`ae9d3604…`)` and remove every `0.67` mention. Change nothing else in `Taskfile.yml`.

## Acceptance

`Taskfile.yml` and `AGENTS.md` name ESS 0.68.0 and its commit, nothing else in the Taskfile changes,
and neither file still names 0.67.0. Each check below must hold:

- S3.2 `grep -c '^  BILLING_REPO: .* --ref ae9d3604ec001b3f0f318e8d961748c388082f60$' Taskfile.yml` prints `1`.
- S3.3 ``grep -c 'ESS 0.68.0 (`ae9d3604…`)' AGENTS.md`` prints `1`, and `grep -c '0\.67' AGENTS.md` prints `0`.
- S3.1 (these files) `git grep -n -e 4579d804 -- Taskfile.yml AGENTS.md` prints nothing.
- S5.2 `git diff $(git merge-base HEAD main) -- Taskfile.yml | grep -E '^[+-][^+-]'` prints exactly two
  lines: the old `BILLING_REPO` line and the new one.
