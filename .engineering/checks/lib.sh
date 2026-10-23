#!/usr/bin/env bash
# Shared vocabulary for the checks under .engineering/checks/story/.
#
# Sourced by every story check; it asserts nothing itself. Modelled on metaharness
# `evals/aep/checks/lib.sh`, and it makes the same two rules mechanical:
#
#   * **A vacuous check is a failed check.** Every row a check declares must be reported; `finish`
#     fails the check if one was not, so a row that fell out of a branch is red, not absent.
#   * **The table prints on every path, including failure.** Nothing here sets `-e` and no helper
#     exits early.
#
# A check also names the unit it decides (`unit`), and `run.sh` fails a check that names none.
set -uo pipefail

CHECKS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$CHECKS_DIR/../.." && pwd)"
cd "$REPO" || exit 1

# Scratch lives in the git-ignored out/, never /tmp.
SCRATCH_BASE="$REPO/out/checks"

# ---- the constants of specification:ess-068-crates ----------------------------------------------
NEW=ae9d3604ec001b3f0f318e8d961748c388082f60
OLD=4579d8049bb5b83749b46059dfc0982de7633485
CRATES=(ess-primitives ess-domain ess-compiler ess-conformance)
FIXTURES=(billing parcel-locker tally)

# ---- the unit ------------------------------------------------------------------------------------

# unit <id> <statement>  — the one unit this check decides. Printed first, read by run.sh.
unit() { printf 'UNIT  %s — %s\n' "$1" "$2"; }

# ---- rows ---------------------------------------------------------------------------------------

declare -A STATEMENT=()
declare -A REPORTED=()
ROW_IDS=()
FAILED=0

# declare_row <id> <statement>
declare_row() {
  STATEMENT["$1"]="$2"
  ROW_IDS+=("$1")
}

# row <id> <exit-status>  — 0 is a pass, anything else is a failure.
row() {
  local id="$1" code="$2"
  if [ -n "${REPORTED[$id]:-}" ]; then
    printf 'FAIL  %-5s reported twice — the check is confused about its own rows\n' "$id"
    FAILED=$((FAILED + 1))
    return
  fi
  REPORTED["$id"]=1
  if [ "$code" -eq 0 ]; then
    printf 'PASS  %-5s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
  else
    printf 'FAIL  %-5s %s\n' "$id" "${STATEMENT[$id]:-<undeclared row>}"
    FAILED=$((FAILED + 1))
  fi
}

# why <text…>  — the reason under the row it belongs to. Printed, never counted.
why() { printf '        ↳ %s\n' "$*"; }

# red_all <reason>  — every not-yet-reported row goes red for one shared reason. Not a skip.
red_all() {
  local reason="$1" id
  for id in "${ROW_IDS[@]}"; do
    [ -n "${REPORTED[$id]:-}" ] && continue
    row "$id" 1
    why "$reason"
  done
}

# finish  — the check's exit status, and the last enforcement of the no-silent-row rule.
finish() {
  local id missing=0
  for id in "${ROW_IDS[@]}"; do
    if [ -z "${REPORTED[$id]:-}" ]; then
      printf 'FAIL  %-5s never reported — a row that did not run is not a row that passed\n' "$id"
      missing=$((missing + 1))
    fi
  done
  [ "$((FAILED + missing))" -eq 0 ]
}

# ---- preconditions ------------------------------------------------------------------------------

# have <command>  — is the tool on PATH.
have() { command -v "$1" >/dev/null 2>&1; }

# scratch <name>  — a fresh directory under out/checks/.
scratch() {
  mkdir -p "$SCRATCH_BASE" || return 1
  mktemp -d "$SCRATCH_BASE/$1.XXXXXX"
}

# base  — the merge base with main, which every "changed by this story" row diffs against.
base() {
  local ref
  for ref in main origin/main; do
    git rev-parse -q --verify "$ref^{commit}" >/dev/null 2>&1 || continue
    git merge-base HEAD "$ref" 2>/dev/null && return 0
  done
  return 1
}

# pinned_to_new  — Cargo.toml pins all four CRATES to NEW (S1.1's predicate, reused as a guard).
pinned_to_new() {
  [ "$(grep -cE '^(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "'"$NEW"'" \}$' Cargo.toml)" = 4 ]
}

# locked_to_new  — every ESS package in Cargo.lock resolves to NEW (S2.1's predicate, as a guard).
locked_to_new() {
  [ "$(grep '^source = "git+https://github.com/beyond10x/ess' Cargo.lock | sort -u)" \
    = "source = \"git+https://github.com/beyond10x/ess?rev=$NEW#$NEW\"" ]
}
