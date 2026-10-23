#!/usr/bin/env bash
# specification:ess-068-crates § S5 — the repository's gate is green on the new pin.
#
# S5.1 and S5.4 run only on the new pin: `task check` is green on 0.67.0 today, and a gate that
# passes before the move decides nothing about it.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S5' 'task check is green on the ESS 0.68.0 pin, with the gate and the tests left as they were'

declare_row S5.1 'task check exits 0 on the NEW pin'
declare_row S5.2 'the only Taskfile.yml line this story changes is BILLING_REPO'
declare_row S5.3 'no #[ignore] added, no #[test] removed, no test file deleted under crates/, tests/, web/test/'
declare_row S5.4 'schema/ and web/dist/ are fresh against their sources (task fresh) on the NEW pin'

on_new=1
pinned_to_new && locked_to_new && on_new=0
if ! have task; then
  row S5.1 1; why 'task is not on PATH'
  row S5.4 1; why 'task is not on PATH'
elif [ "$on_new" -ne 0 ]; then
  row S5.1 1; why 'Cargo.toml/Cargo.lock are not on NEW (S1.1, S2.1) — the gate on the old pin decides nothing'
  row S5.4 1; why 'Cargo.toml/Cargo.lock are not on NEW (S1.1, S2.1)'
else
  work=$(scratch s5)
  task check > "$work/check.log" 2>&1; row S5.1 $?
  why "log: ${work#"$REPO"/}/check.log"
  task fresh > "$work/fresh.log" 2>&1; row S5.4 $?
  why "log: ${work#"$REPO"/}/fresh.log"
fi

if b=$(base); then
  other=$(git diff -U0 "$b" -- Taskfile.yml | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) ' \
    | grep -vE '^[+-]  BILLING_REPO: ')
  [ -z "$other" ]; row S5.2 $?
  [ -z "$other" ] || why "$(printf '%s' "$other" | head -n 4 | tr '\n' ' ')"

  d=$(git diff -U0 "$b" -- crates/ tests/ web/test/)
  ignored=$(printf '%s\n' "$d" | grep -E '^\+[^+].*#\[ignore')
  removed=$(printf '%s\n' "$d" | grep -E '^-[^-].*#\[test\]')
  deleted=$(git diff --diff-filter=D --name-only "$b" -- crates/ tests/ web/test/)
  [ -z "$ignored$removed$deleted" ]; row S5.3 $?
  [ -z "$ignored" ] || why "adds #[ignore]: $(printf '%s' "$ignored" | head -n 3 | tr '\n' ' ')"
  [ -z "$removed" ] || why "removes #[test]: $(printf '%s' "$removed" | head -n 3 | tr '\n' ' ')"
  [ -z "$deleted" ] || why "deletes: $(printf '%s' "$deleted" | head -n 3 | tr '\n' ' ')"
  # The rest of S5.3 — that each crates/ hunk is forced by the 0.68.0 API — is a reviewer's call.
else
  row S5.2 1; why 'no merge base with main to diff against'
  row S5.3 1; why 'no merge base with main to diff against'
fi

finish
