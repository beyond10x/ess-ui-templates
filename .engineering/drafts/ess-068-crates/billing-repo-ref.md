# Task: Point the billing page's source links at the ESS 0.68.0 commit

## Outcome

The billing page links to `examples/billing` at ESS 0.68.0: `Taskfile.yml` var `BILLING_REPO` ends
`--ref ae9d3604ec001b3f0f318e8d961748c388082f60`, and that line is the only change to `Taskfile.yml`.

## Context

- `specification:ess-068-crates` § S3.2, § S5.2.
- `Taskfile.yml:11` ends `--ref 4579d8049bb5b83749b46059dfc0982de7633485`.

## Acceptance

- S3.2: `grep -c '^  BILLING_REPO: .* --ref ae9d3604ec001b3f0f318e8d961748c388082f60$' Taskfile.yml` prints `1`.
- `grep -c 4579d804 Taskfile.yml` prints `0`.
- S5.2: `git diff -U0 <base> -- Taskfile.yml | grep -E '^[-+][^-+]'` prints exactly two lines, both
  starting `  BILLING_REPO:` (one `-`, one `+`), where `<base>` is `git merge-base HEAD main`.

## Scope

- `Taskfile.yml:11` — cited.

## Out of scope

- Every other line of `Taskfile.yml`, in particular the `check` and `fixtures` tasks (S5.2).
