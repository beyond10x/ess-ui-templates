#!/usr/bin/env bash
# The checks a driven run records (aep `drivers/development/checks.yaml`): every story's own
# checks under .engineering/checks/story/, then the repository gate. Exit 0 only when all pass.
set -uo pipefail
cd "$(dirname "$0")/../.."

failed=0
for check in .engineering/checks/story/*.sh; do
  [ -e "$check" ] || continue
  if bash "$check"; then
    printf 'ok    %s\n' "$check"
  else
    printf 'FAIL  %s\n' "$check"
    failed=1
  fi
done

mkdir -p out
if task check > out/checks-task-check.log 2>&1; then
  printf 'ok    task check\n'
else
  printf 'FAIL  task check (out/checks-task-check.log)\n'
  failed=1
fi

exit "$failed"
