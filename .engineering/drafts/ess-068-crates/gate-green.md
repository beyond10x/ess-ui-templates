# Task: task check and the pages are green on the ESS 0.68.0 tree

## Outcome

With the crates pinned, the Taskfile and AGENTS.md updated and the fixtures regenerated, the
repository's gate passes unweakened, the pages build, and nothing tracked names ESS 0.67.0.

## Context

- `specification:ess-068-crates` § S3.1, § S5.1–S5.3, § S6, § D4.
- Integration task over the four sibling tasks; it runs on their combined tree.

## Acceptance

- S3.1: `git grep -n -e 4579d8049bb5b83749b46059dfc0982de7633485 -e 4579d804 -- ':!.engineering'` prints nothing.
- S5.1: `task check` exits 0.
- S5.2: `git diff -U0 <base> -- Taskfile.yml | grep -E '^[-+][^-+]'` prints only the two
  `  BILLING_REPO:` lines, where `<base>` is `git merge-base HEAD main`.
- S5.3: `git diff <base> -- crates/ tests/ web/test/` adds no `#[ignore]`, removes no `#[test]`
  function and deletes no test file.
- S6.1: `task page` exits 0, and `test -s` succeeds on `out/billing.html`, `out/parcel-locker.html`
  and `out/billing.data.json`.
- S6.2: `out/billing.data.json` validates against `schema/ess-ui-presentation.schema.json`
  (`cargo test --locked` and `task web:test`, both inside `task check`).
- Decline D4 if passing `task check` would require writing semantics ESS executes.

## Scope

- No surface of its own. Fixes needed to turn the gate green land under `crates/**/*.rs`, inside the
  limits of S5.3, and run after `task:ess-068-pin-crates`, never alongside it.

## Out of scope

- Constructs the page lists as not rendered — `story:draw-ess-068-constructs`.
- Replay failures in `synthesize.log` — `story:check-replay-green`.
