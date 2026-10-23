# Task: Pin the four ESS crates to ESS 0.68.0 in the manifest and lockfile

## Outcome

`ess-ui` builds against ESS 0.68.0 (`ae9d3604ec001b3f0f318e8d961748c388082f60`): the workspace
manifest and the lockfile name that commit for `ess-primitives`, `ess-domain`, `ess-compiler` and
`ess-conformance`, and nothing else.

## Context

- `specification:ess-068-crates` § S1, § S2, § S5.3, § S5.4, § D1.
- `Cargo.toml:14-17` pins the four crates to `4579d8049bb5b83749b46059dfc0982de7633485` (0.67.0).

## Acceptance

Each line is one check, run from the repository root on the task's tree.

- Precondition (D1): `git ls-remote https://github.com/beyond10x/ess 'refs/tags/0.68.0^{}'` prints
  `ae9d3604ec001b3f0f318e8d961748c388082f60` as its first field; otherwise the task declines.
- S1.1: `grep -cE '^(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "ae9d3604ec001b3f0f318e8d961748c388082f60" \}$' Cargo.toml` prints `4`.
- S1.2: `grep -n 'beyond10x/ess"' Cargo.toml | grep -v 'rev = "ae9d3604ec001b3f0f318e8d961748c388082f60"'` prints nothing.
- S1.3: `grep -rn 'beyond10x/ess' crates/*/Cargo.toml` prints nothing.
- S2.1: `grep '^source = "git+https://github.com/beyond10x/ess' Cargo.lock | sort -u` prints exactly
  `source = "git+https://github.com/beyond10x/ess?rev=ae9d3604ec001b3f0f318e8d961748c388082f60#ae9d3604ec001b3f0f318e8d961748c388082f60"`.
- S2.2: for each crate `c` of the four, `grep -c "^name = \"$c\"$" Cargo.lock` prints `1`.
- S2.3: `cargo metadata --locked --format-version 1 > /dev/null` exits 0.
- Build: `cargo build --locked -p ess-ui` exits 0.
- S5.4: `task fresh` exits 0 (a regenerated `schema/ess-ui-presentation.schema.json` is committed if
  the 0.68.0 types change it).
- S5.3: `git diff <base> -- crates/` adds no `#[ignore]`, removes no `#[test]` function and deletes no
  test file, where `<base>` is `git merge-base HEAD main`.

## Scope

- `Cargo.toml`, `Cargo.lock` — cited (spec § Facts, `Cargo.toml:14-17`).
- `crates/**/*.rs` — inferred: only hunks the 0.68.0 API forces for `cargo build --locked -p ess-ui`.
- `schema/ess-ui-presentation.schema.json`, `web/dist/` — inferred: only if `task fresh` reports them stale.

## Out of scope

- `Taskfile.yml`, `AGENTS.md`, `fixtures/` — sibling tasks.
- `task check` as a whole — `task:ess-068-gate-green`.
