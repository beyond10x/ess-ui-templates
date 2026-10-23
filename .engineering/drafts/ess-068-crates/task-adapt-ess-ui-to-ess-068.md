# Task: Make ess-ui compile, lint clean and pass its tests against the ESS 0.68.0 crates

Part of `story:ess-068-crates`; covers `specification:ess-068-crates` S5.3. Depends on
`task:pin-ess-crates`.

## Scope

- `crates/ess-ui/src/**` (inferred: only where the 0.68.0 API forces a change; possibly no change)

## Work

Build on the new pin. Where the 0.68.0 crates change an API `ess-ui` calls, make the smallest change
that compiles. Do not add semantics ESS executes (`AGENTS.md` § Rules). If compiling would need that,
the task declines and cites D4. Do not touch `Cargo.toml`, `Cargo.lock`, `fixtures/`, `schema/` or
`web/`.

## Acceptance

On the tree with `task:pin-ess-crates` applied, `ess-ui` builds, lints clean and passes its tests with
no test weakened. Each check below must hold:

- `cargo build --locked -p ess-ui` exits 0.
- `cargo clippy --locked --all-targets -- -D warnings` exits 0.
- `cargo fmt --all --check` exits 0.
- S5.3 `git diff <base> -- crates/ tests/ web/test/` adds no `#[ignore]` line, removes no `#[test]`
  line and shows no `deleted file`. Concretely,
  `git diff <base> -- crates/ tests/ web/test/ | grep -E '^\+.*#\[ignore|^-.*#\[test\]|^deleted file'`
  prints nothing. `<base>` is `git merge-base HEAD main`.

`cargo test --locked` is not required here, because some tests may read `fixtures/`, which
`task:regenerate-fixtures-ess-068` owns. `task:ess-068-gate-green` checks it.
