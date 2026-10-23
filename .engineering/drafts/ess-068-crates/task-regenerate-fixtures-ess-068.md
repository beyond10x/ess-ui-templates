# Task: Regenerate the fixtures with the ess 0.68.0 CLI and commit them

Part of `story:ess-068-crates`; covers `specification:ess-068-crates` S4. This task is independent of
the Cargo pin because `task fixtures` runs the `ess` binary, not the linked crates.

## Scope

- `fixtures/{billing,parcel-locker,tally}/{ir.json,suite.json,authored.json,synthesize.log}` (cited, `Taskfile.yml:19-29`)

## Precondition

D2: `ess --version` ends in `0.68.0`. Otherwise the task declines and cites D2.

## Work

Run `task fixtures` and commit the four generated files per fixture. Do not edit
`fixtures/*/spec/**` or `fixtures/*/scenarios/**`. If ESS 0.68.0 refuses an unchanged spec, the task
declines and cites D3.

## Acceptance

The committed fixtures are exactly what `ess` 0.68.0 writes from the unchanged fixture
specifications, and regenerating them changes nothing. Each check below must hold:

- S4.1 the last whitespace-separated field of `ess --version` is `0.68.0`.
- S4.2 for each `f` in `billing parcel-locker tally`,
  `git ls-files --error-unmatch fixtures/$f/ir.json fixtures/$f/suite.json fixtures/$f/authored.json fixtures/$f/synthesize.log` exits 0.
- S4.3 `task fixtures` exits 0 and then `git status --porcelain -- fixtures/` prints nothing. Running
  both commands a second time gives the same result.
- S4.4 for every `p` in `git ls-files 'fixtures/*/ir.json' 'fixtures/*/suite.json' 'fixtures/*/authored.json'`,
  `jq empty "$p"` exits 0.
- S4.5 `git diff --name-only $(git merge-base HEAD main) -- 'fixtures/*/spec' 'fixtures/*/scenarios'` prints nothing.

This task promises no count of passed or failed steps in `synthesize.log` (`story:check-replay-green`).
