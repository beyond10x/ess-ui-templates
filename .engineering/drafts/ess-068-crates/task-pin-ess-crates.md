# Task: Pin the four ESS crates to ESS 0.68.0 in Cargo.toml and Cargo.lock

Part of `story:ess-068-crates`; covers `specification:ess-068-crates` S1 and S2.

## Scope

- `Cargo.toml` lines 14-17 (`[workspace.dependencies]`, cited)
- `Cargo.lock` (cited)

## Precondition

D1: `git ls-remote https://github.com/beyond10x/ess 'refs/tags/0.68.0^{}'` prints
`ae9d3604ec001b3f0f318e8d961748c388082f60` as its first field. Otherwise the task declines and cites D1.

## Work

Replace `rev = "4579d8049bb5b83749b46059dfc0982de7633485"` with
`rev = "ae9d3604ec001b3f0f318e8d961748c388082f60"` on the four ESS lines, then
`cargo update -p ess-primitives -p ess-domain -p ess-compiler -p ess-conformance`. Edit no source
under `crates/`. That is `task:adapt-ess-ui-to-ess-068`.

## Acceptance

`Cargo.toml` pins all four ESS crates to `ae9d3604…`, and `Cargo.lock` resolves every ESS package to
that commit and to nothing else. Each check below must hold:

- S1.1 `grep -cE '^(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "ae9d3604ec001b3f0f318e8d961748c388082f60" \}$' Cargo.toml` prints `4`.
- S1.2 `grep -n 'beyond10x/ess"' Cargo.toml | grep -v 'rev = "ae9d3604ec001b3f0f318e8d961748c388082f60"'` prints nothing.
- S1.3 `grep -rn 'beyond10x/ess' crates/*/Cargo.toml` prints nothing.
- S2.1 `grep '^source = "git+https://github.com/beyond10x/ess' Cargo.lock | sort -u` prints exactly
  `source = "git+https://github.com/beyond10x/ess?rev=ae9d3604ec001b3f0f318e8d961748c388082f60#ae9d3604ec001b3f0f318e8d961748c388082f60"`.
- S2.2 for each `c` in `ess-primitives ess-domain ess-compiler ess-conformance`,
  `grep -c "^name = \"$c\"$" Cargo.lock` prints `1`.
- S2.3 `cargo metadata --locked --format-version 1 > /dev/null` exits 0.

Whether `ess-ui` still compiles is not part of this task's acceptance.
