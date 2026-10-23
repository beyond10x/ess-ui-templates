---
format: aep.planning-md/3
id: specification:ess-068-crates
kind: specification
status: approved
title: ESS crates and fixtures on ESS 0.68.0
relations:
- specifies: story:ess-068-crates
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-23T14:07:15Z", actor: "human:timo", revision: 3}
- {from: "in_review", to: "approved", at: "2026-10-23T14:07:15Z", actor: "human:timo", revision: 4}
---
# Specification: ESS crates and fixtures on ESS 0.68.0

Specifies `story:ess-068-crates`. Every statement below is a check: the command after it, run from
the repository root on the change's tree, must produce what the statement says. Where a statement
names "nothing", the command's output must be empty.

## Constants

| Name | Value |
|---|---|
| `NEW` | `ae9d3604ec001b3f0f318e8d961748c388082f60` (ESS tag `0.68.0`, peeled) |
| `OLD` | `4579d8049bb5b83749b46059dfc0982de7633485` (ESS 0.67.0) |
| `CRATES` | `ess-primitives`, `ess-domain`, `ess-compiler`, `ess-conformance` |
| `FIXTURES` | `billing`, `parcel-locker`, `tally` (the `for:` list of `Taskfile.yml` task `fixtures`) |

`NEW` is the ESS tag's commit, not a lookalike:
`git ls-remote https://github.com/beyond10x/ess 'refs/tags/0.68.0^{}'` prints `NEW` as its first field.

## Required behaviour

### S1. The manifest pins every ESS crate to NEW

- S1.1 Each of the four `CRATES` has exactly one line in `[workspace.dependencies]` of `Cargo.toml`,
  with `git = "https://github.com/beyond10x/ess"` and `rev = "NEW"`.
  Check: `grep -cE '^(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "ae9d3604ec001b3f0f318e8d961748c388082f60" \}$' Cargo.toml` prints `4`.
- S1.2 No ESS dependency in `Cargo.toml` uses `branch`, `tag` or a `rev` other than `NEW`.
  Check: `grep -n 'beyond10x/ess"' Cargo.toml | grep -v 'rev = "ae9d3604ec001b3f0f318e8d961748c388082f60"'` prints nothing.
- S1.3 Every crate manifest under `crates/` takes the ESS crates from the workspace and pins nothing
  of its own. Check: `grep -rn 'beyond10x/ess' crates/*/Cargo.toml` prints nothing.

### S2. The lockfile resolves every ESS package to NEW, and only to NEW

- S2.1 Every `[[package]]` in `Cargo.lock` whose `source` begins `git+https://github.com/beyond10x/ess`
  has the source `git+https://github.com/beyond10x/ess?rev=NEW#NEW`.
  Check: `grep '^source = "git+https://github.com/beyond10x/ess' Cargo.lock | sort -u` prints exactly one line,
  `source = "git+https://github.com/beyond10x/ess?rev=ae9d3604ec001b3f0f318e8d961748c388082f60#ae9d3604ec001b3f0f318e8d961748c388082f60"`.
- S2.2 Each of the four `CRATES` appears as a package name in `Cargo.lock` exactly once (no second
  copy left at `OLD`). Check: for each crate `c`, `grep -c "^name = \"$c\"$" Cargo.lock` prints `1`.
- S2.3 The lockfile is current: `cargo metadata --locked --format-version 1 > /dev/null` exits 0.

### S3. Nothing in the tracked tree still names ESS 0.67.0

- S3.1 `git grep -n -e 4579d8049bb5b83749b46059dfc0982de7633485 -e 4579d804 -- ':!.engineering'` prints nothing.
- S3.2 `Taskfile.yml` var `BILLING_REPO` ends `--ref ae9d3604ec001b3f0f318e8d961748c388082f60`.
  Check: `grep -c '^  BILLING_REPO: .* --ref ae9d3604ec001b3f0f318e8d961748c388082f60$' Taskfile.yml` prints `1`.
- S3.3 `AGENTS.md` § Rules states the pin as ESS 0.68.0 and the commit's prefix.
  Check: `grep -c 'ESS 0.68.0 (`ae9d3604…`)' AGENTS.md` prints `1`, and `grep -c '0\.67' AGENTS.md` prints `0`.

### S4. The fixtures are the output of `ess` 0.68.0, and reproducible

- S4.1 The `ess` binary used is 0.68.0: `ess --version` prints a version of exactly `0.68.0`
  (its last whitespace-separated field).
- S4.2 For each fixture `f` in `FIXTURES`, the files `fixtures/f/ir.json`, `fixtures/f/suite.json`,
  `fixtures/f/authored.json` and `fixtures/f/synthesize.log` are tracked:
  `git ls-files --error-unmatch fixtures/f/ir.json fixtures/f/suite.json fixtures/f/authored.json fixtures/f/synthesize.log` exits 0.
- S4.3 Regeneration is a fixed point: on the committed tree with the `ess` of S4.1 on `PATH`,
  `task fixtures` exits 0 and afterwards `git status --porcelain -- fixtures/` prints nothing.
  (This is what "the diff is committed" means, and it holds on every rerun: running `task fixtures`
  a second time also leaves `git status --porcelain -- fixtures/` empty.)
- S4.4 Every `ir.json`, `suite.json` and `authored.json` under `fixtures/` parses as JSON:
  for each file `p`, `jq empty p` exits 0.
- S4.5 The fixture specifications themselves (`fixtures/*/spec/**`, `fixtures/*/scenarios/**`) are
  not changed by this story: `git diff --name-only <base> -- 'fixtures/*/spec' 'fixtures/*/scenarios'`
  prints nothing, where `<base>` is the merge base with `main`. If ESS 0.68.0 refuses a fixture spec
  unchanged, that refusal is the reason to decline (see Declining), not a licence to edit the spec.

### S5. The repository's gate is green on the new pin

- S5.1 `task check` exits 0. That includes, as the Taskfile declares it today: `cargo fmt --all --check`,
  `cargo clippy --locked --all-targets -- -D warnings` (0 errors, 0 warnings), `cargo test --locked`
  (0 failed), `task fresh`, `task tokens:check`, `task web:test` and `task scan`.
- S5.2 The `check` task's command list in `Taskfile.yml` is unchanged by this story — no step
  removed, skipped or weakened to get S5.1 green: `git diff <base> -- Taskfile.yml` changes only the
  `BILLING_REPO` line.
- S5.3 No test is disabled or deleted to get S5.1 green: `git diff <base> -- crates/ tests/ web/test/`
  adds no `#[ignore]`, removes no `#[test]` function and deletes no test file. Source changes under
  `crates/` are permitted only where the 0.68.0 crates' API forces them, and each such hunk must be
  needed for `cargo build --locked -p ess-ui` to succeed.
- S5.4 `schema/` and `web/dist/` remain fresh against their sources (implied by `task fresh` in S5.1);
  if the 0.68.0 types change the generated schema, the regenerated `schema/ess-ui-presentation.schema.json`
  is committed.

### S6. The pages build from the regenerated fixtures

- S6.1 `task page` exits 0 and writes `out/billing.html`, `out/parcel-locker.html` and
  `out/billing.data.json`, each non-empty (`test -s` on each).
- S6.2 `out/billing.data.json` validates against `schema/ess-ui-presentation.schema.json`
  (covered by `cargo test --locked` and `task web:test` in S5.1; stated here so a verifier
  can run it on its own).

## Not required (out of scope)

- Drawing the constructs the page lists as not rendered — `story:draw-ess-068-constructs`.
- Driving the downstream replay to zero failed synthesized steps — `story:check-replay-green`.
  This story does not promise any count of failed or passed steps in `synthesize.log`; it promises
  only that the log is the one `ess` 0.68.0 writes (S4.3).
- Any change to the docs-system pin, fonts or tokens.

## Declining

The run declines, with this specification as the record, when any of these is observed:

- D1. `git ls-remote https://github.com/beyond10x/ess 'refs/tags/0.68.0^{}'` does not print `NEW`
  (the tag moved or does not exist).
- D2. An `ess` 0.68.0 binary is not available to the run, so S4.1/S4.3 cannot be observed.
- D3. ESS 0.68.0 refuses one of the unchanged fixture specifications in `task fixtures` (S4.5).
- D4. Making S5.1 pass would require writing semantics ESS executes, contrary to `AGENTS.md` § Rules
  ("Never write semantics ESS executes").

## Facts this specification was written from

- `Cargo.toml:14-17` pins the four `CRATES` to `OLD`.
- `Taskfile.yml:11` `BILLING_REPO` ends `--ref OLD`; `Taskfile.yml:19-29` is task `fixtures` over
  `billing`, `parcel-locker`, `tally`; `Taskfile.yml:128-137` is task `check`.
- `AGENTS.md:24-25` states the pin as ESS 0.67.0 (`4579d804…`) and that moving it means
  `task fixtures` and the full `task check`.
- `NEW` is quoted from `story:ess-068-crates` § Context; the `git ls-remote` check under Constants re-derives it.
