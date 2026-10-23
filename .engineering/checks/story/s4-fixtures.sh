#!/usr/bin/env bash
# specification:ess-068-crates § S4 — the fixtures are the output of `ess` 0.68.0, and reproducible.
#
# S4.3 regenerates in a scratch copy of fixtures/ and Taskfile.yml, never in place: a check that
# rewrites the tree it is judging would make its own verdict.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S4' 'fixtures/ is what ess 0.68.0 writes from the unchanged fixture specs, and regenerating it changes nothing'

declare_row S4.1 'the ess on PATH is exactly 0.68.0'
declare_row S4.2 'every fixture tracks ir.json, suite.json, authored.json and synthesize.log'
declare_row S4.3 'task fixtures with that ess is a fixed point of the committed fixtures/, twice over'
declare_row S4.4 'every ir.json, suite.json and authored.json under fixtures/ parses as JSON'
declare_row S4.5 'no fixture spec or scenario changed since the merge base with main'

# ---- S4.1 ---------------------------------------------------------------------------------------
ess_ok=1
if have ess; then
  v=$(ess --version 2>/dev/null | awk '{print $NF}')
  [ "$v" = 0.68.0 ] && ess_ok=0
  row S4.1 "$ess_ok"
  [ "$ess_ok" -eq 0 ] || why "ess --version reports '${v:-nothing}'"
else
  row S4.1 1; why 'ess is not on PATH (decline D2 if no 0.68.0 binary exists)'
fi

# ---- S4.2 ---------------------------------------------------------------------------------------
missing=()
for f in "${FIXTURES[@]}"; do
  git ls-files --error-unmatch fixtures/$f/ir.json fixtures/$f/suite.json \
    fixtures/$f/authored.json fixtures/$f/synthesize.log > /dev/null 2>&1 || missing+=("$f")
done
[ "${#missing[@]}" -eq 0 ]; row S4.2 $?
[ "${#missing[@]}" -eq 0 ] || why "untracked outputs in: ${missing[*]}"

# ---- S4.3 ---------------------------------------------------------------------------------------
if [ "$ess_ok" -ne 0 ]; then
  row S4.3 1; why 'needs the ess of S4.1 — a fixed point under another ess says nothing about 0.68.0'
elif ! have task; then
  row S4.3 1; why 'task is not on PATH'
elif ! work=$(scratch s4) || [ -z "$work" ]; then
  row S4.3 1; why 'could not make a scratch directory under out/checks'
else
  dirty=$(git status --porcelain -- fixtures/)
  cp -a fixtures Taskfile.yml "$work"/
  result=0
  for pass in 1 2; do
    if ! task --dir "$work" fixtures > "$work/task-$pass.log" 2>&1; then
      result=1; why "task fixtures (run $pass) failed: $(tail -n 2 "$work/task-$pass.log" | tr '\n' ' ')"
      break
    fi
    if ! d=$(diff -rq fixtures "$work/fixtures"); then
      result=1; why "run $pass changed: $(printf '%s' "$d" | head -n 4 | tr '\n' ' ')"
      break
    fi
  done
  [ -z "$dirty" ] || { result=1; why "fixtures/ is not committed: $(printf '%s' "$dirty" | head -n 4 | tr '\n' ' ')"; }
  row S4.3 "$result"
fi

# ---- S4.4 ---------------------------------------------------------------------------------------
if have jq; then
  broken=()
  while IFS= read -r p; do
    jq empty "$p" > /dev/null 2>&1 || broken+=("$p")
  done < <(find fixtures -type f \( -name ir.json -o -name suite.json -o -name authored.json \))
  [ "${#broken[@]}" -eq 0 ]; row S4.4 $?
  [ "${#broken[@]}" -eq 0 ] || why "${broken[*]}"
else
  row S4.4 1; why 'jq is not on PATH'
fi

# ---- S4.5 ---------------------------------------------------------------------------------------
if b=$(base); then
  changed=$(git diff --name-only "$b" -- 'fixtures/*/spec' 'fixtures/*/scenarios')
  [ -z "$changed" ]; row S4.5 $?
  [ -z "$changed" ] || why "$(printf '%s' "$changed" | head -n 4 | tr '\n' ' ') (decline D3 rather than edit a spec)"
else
  row S4.5 1; why 'no merge base with main to diff against'
fi

finish
