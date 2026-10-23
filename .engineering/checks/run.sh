#!/usr/bin/env bash
# The checks a driven run records (aep `drivers/development/checks.yaml`): every story's own
# checks under .engineering/checks/story/, then the repository gate. Exit 0 only when all pass.
#
# Modelled on metaharness `evals/aep/checks/run-checks.sh`. A story check names the unit it decides
# (a `UNIT` line, from lib.sh) and prints one `PASS`/`FAIL` row per statement. Besides its own
# exit code, a check fails here when it names no unit, produces no rows — a table with nothing in
# it checks nothing — or exits 0 beside a red row.
set -uo pipefail
cd "$(dirname "$0")/../.."

mkdir -p out/checks
OUT="$(mktemp out/checks/run.XXXXXX)" || exit 1
trap 'rm -f "$OUT"' EXIT

failed=0
total_pass=0
total_fail=0
for check in .engineering/checks/story/*.sh; do
  [ -e "$check" ] || continue
  printf '\n== %s ==\n' "$check"
  # `bash "$check"`, never `"$check"`: a missing execute bit is not a verdict about the unit.
  bash "$check" > "$OUT" 2>&1
  status=$?
  cat "$OUT"

  pass=$(grep -c '^PASS ' "$OUT")
  fail=$(grep -c '^FAIL ' "$OUT")
  total_pass=$((total_pass + pass))
  total_fail=$((total_fail + fail))

  if ! grep -q '^UNIT ' "$OUT"; then
    printf 'FAIL  ----  %s names no unit it decides\n' "$check"
    status=1
  fi
  if [ "$((pass + fail))" -eq 0 ]; then
    printf 'FAIL  ----  %s produced no rows (exit %s) — a check that asserts nothing is not green\n' \
      "$check" "$status"
    status=1
  elif [ "$status" -eq 0 ] && [ "$fail" -gt 0 ]; then
    printf 'FAIL  ----  %s exited 0 with %s red row(s) — its exit code disagrees with its table\n' \
      "$check" "$fail"
    status=1
  fi

  if [ "$status" -eq 0 ]; then
    printf 'ok    %s\n' "$check"
  else
    printf 'FAIL  %s\n' "$check"
    failed=1
  fi
done

printf '\n== story checks: %s pass, %s fail ==\n' "$total_pass" "$total_fail"

if task check > out/checks-task-check.log 2>&1; then
  printf 'ok    task check\n'
else
  printf 'FAIL  task check (out/checks-task-check.log)\n'
  failed=1
fi

exit "$failed"
