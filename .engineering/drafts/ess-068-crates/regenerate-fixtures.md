# Task: Regenerate the fixtures with ess 0.68.0

## Outcome

`fixtures/{billing,parcel-locker,tally}/{ir,suite,authored}.json` and `synthesize.log` are what
`ess` 0.68.0 writes from the unchanged fixture specifications, and rerunning `task fixtures`
changes nothing.

## Context

- `specification:ess-068-crates` § S4, § D2, § D3.
- `Taskfile.yml:19-29` is task `fixtures`. It runs the `ess` CLI on `PATH` and does not link the
  workspace crates, so this task does not depend on `task:ess-068-pin-crates`.

## Acceptance

- S4.1 (else decline D2): `ess --version | awk '{print $NF}'` prints `0.68.0`.
- S4.3: with that `ess` on `PATH`, `task fixtures` exits 0 and afterwards
  `git status --porcelain -- fixtures/` prints nothing; a second `task fixtures` again leaves it empty.
- S4.2: for each `f` in `billing parcel-locker tally`,
  `git ls-files --error-unmatch fixtures/$f/ir.json fixtures/$f/suite.json fixtures/$f/authored.json fixtures/$f/synthesize.log` exits 0.
- S4.4: for each `p` in `fixtures/*/{ir,suite,authored}.json`, `jq empty "$p"` exits 0.
- S4.5 (else decline D3): `git diff --name-only <base> -- 'fixtures/*/spec' 'fixtures/*/scenarios'`
  prints nothing, where `<base>` is `git merge-base HEAD main`.

## Scope

- `fixtures/*/ir.json`, `fixtures/*/suite.json`, `fixtures/*/authored.json`,
  `fixtures/*/synthesize.log` — cited (spec § S4.2).

## Out of scope

- `fixtures/*/spec/**`, `fixtures/*/scenarios/**` — must not change (S4.5).
- Any count of passed or failed steps in `synthesize.log` — `story:check-replay-green`.
