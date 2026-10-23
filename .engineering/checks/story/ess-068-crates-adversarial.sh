#!/usr/bin/env bash
# Adversarial checks for story:ess-068-crates — the holes between the statements of
# specification:ess-068-crates, each a way the change could be wrong while S1–S5 stay green:
#
#   X1  S2.1 checks the *set* of ESS sources and S2.2 counts package *names*; neither binds a crate to
#       its source, so a crate taken from a path (a `[patch]`) beside three on NEW passes both.
#   X2  The same, as cargo resolves it: a `[patch]` or `paths` override in `.cargo/config*` (tracked
#       or the user's own) changes what links without touching Cargo.toml.
#   X3  S1.2 only sees `beyond10x/ess"`; `ess.git`, ssh and scp-style URLs pass it at any rev.
#   X4  S3 is titled "nothing names ESS 0.67.0" but S3.1 greps for the hash only.
#   X5  S5.2 protects the check task in Taskfile.yml, but `[workspace.lints]` in Cargo.toml — what
#       `clippy -D warnings` enforces — can be weakened to turn S5.1 green.
#   X6  Lint, format and toolchain configuration files can do the same from outside Cargo.toml.
#   X7  S5.3 forbids `#[ignore]` and removed `#[test]`s, not `cfg(any())`, `allow(...)` or friends.
#   X8  S5.3: no fewer `#[test]` functions than the base, and no test file under web/test/ deleted.
# shellcheck source=../lib.sh
. "$(dirname "$0")/../lib.sh"

unit story:ess-068-crates "the ESS crates link ESS 0.68.0 and the gate was not weakened to say so"

declare_row X1 "Cargo.lock: each ESS crate's own [[package]] has source rev=NEW, none from a path"
declare_row X2 "cargo resolves each ESS crate to NEW, and no [patch]/paths override names ESS"
declare_row X3 "every ESS git dependency in any manifest, in any URL form, has rev = NEW"
declare_row X4 "no tracked file outside .engineering names ESS 0.67 in words"
declare_row X5 "Cargo.toml differs from the base only in the four ESS lines, OLD -> NEW"
declare_row X6 "no clippy/rustfmt/toolchain/.cargo configuration changed or added"
declare_row X7 "no added line under crates/ or tests/ allows a lint or compiles a test away"
declare_row X8 "no fewer #[test] functions than the base, and no web/test/ file deleted"

WANT="git+https://github.com/beyond10x/ess?rev=$NEW#$NEW"

# lock_sources <crate>  — the source of every [[package]] named <crate> in Cargo.lock, `<none>` for
# a path package.
lock_sources() {
  awk -v c="\"$1\"" '
    function emit() { if (n == c) print (s == "" ? "<none>" : s) }
    /^\[\[package\]\]$/ { emit(); n = ""; s = ""; next }
    /^name = /   { n = $3 }
    /^source = / { s = $3; gsub(/"/, "", s) }
    END { emit() }' Cargo.lock
}

# ---- X1 ------------------------------------------------------------------------------------------
x1=0
for c in "${CRATES[@]}"; do
  got="$(lock_sources "$c")"
  if [ "$got" != "$WANT" ]; then
    x1=1
    why "$c: ${got:-<absent>}"
  fi
done
row X1 "$x1"

# ---- X2 ------------------------------------------------------------------------------------------
x2=0
if ! have cargo || ! have jq; then
  x2=1
  why "cargo and jq are needed to read the resolved graph"
elif ! meta="$(cargo metadata --locked --format-version 1 2>/dev/null)"; then
  x2=1
  why "cargo metadata --locked failed"
else
  for c in "${CRATES[@]}"; do
    got="$(jq -r --arg c "$c" '[.packages[] | select(.name == $c) | (.source // "<path>")] | join(" ")' <<<"$meta")"
    if [ "$got" != "$WANT" ]; then
      x2=1
      why "$c resolves to: ${got:-<absent>}"
    fi
  done
fi
for f in Cargo.toml crates/*/Cargo.toml .cargo/config .cargo/config.toml; do
  [ -f "$f" ] || continue
  if grep -nE '^[[:space:]]*(\[patch|paths[[:space:]]*=)' "$f" >/dev/null; then
    x2=1
    why "$f declares a [patch] or paths override"
  fi
done
row X2 "$x2"

# ---- X3 ------------------------------------------------------------------------------------------
# `beyond10x/ess` followed by `.git`, `"`, `/` or end of line — never `ess-ui-templates`.
x3=0
stray="$(grep -nHE 'beyond10x/ess(\.git)?(["/]|$)' Cargo.toml crates/*/Cargo.toml 2>/dev/null \
  | grep -vE '^Cargo\.toml:[0-9]+:(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "'"$NEW"'" \}$')"
if [ -n "$stray" ]; then
  x3=1
  while IFS= read -r l; do why "$l"; done <<<"$stray"
fi
row X3 "$x3"

# ---- X4 ------------------------------------------------------------------------------------------
x4=0
named="$(git grep -nIiE '(^|[^a-z0-9])ess[^a-z0-9]{0,3}v?0\.67([^0-9]|$)' -- ':!.engineering' 2>/dev/null)"
if [ -n "$named" ]; then
  x4=1
  while IFS= read -r l; do why "$l"; done <<<"$named"
fi
row X4 "$x4"

# ---- X5–X8 diff against the base ----------------------------------------------------------------
if ! b="$(base)"; then
  red_all "no merge base with main — nothing to diff against"
  finish
  exit $?
fi

x5=0
changed="$(git diff -U0 "$b" -- Cargo.toml | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) ' \
  | grep -vE '^-(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "'"$OLD"'" \}$' \
  | grep -vE '^\+(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "'"$NEW"'" \}$')"
if [ -n "$changed" ]; then
  x5=1
  while IFS= read -r l; do why "$l"; done <<<"$changed"
fi
row X5 "$x5"

x6=0
config=(clippy.toml .clippy.toml rustfmt.toml .rustfmt.toml rust-toolchain rust-toolchain.toml .cargo
  crates/*/clippy.toml crates/*/rustfmt.toml crates/*/.cargo)
touched="$( { git diff --name-only "$b" -- "${config[@]}"; git ls-files -o --exclude-standard -- "${config[@]}"; } 2>/dev/null | sort -u)"
if [ -n "$touched" ]; then
  x6=1
  while IFS= read -r l; do why "$l"; done <<<"$touched"
fi
row X6 "$x6"

x7=0
added="$(git diff -U0 "$b" -- crates tests | grep -E '^\+' | grep -vE '^\+\+\+ ' \
  | grep -E '#!?\[(allow|expect)\(|#\[ignore|cfg\((any\(\)|FALSE|false|not\(test\))\)|cfg_attr\([^)]*(ignore|allow)')"
if [ -n "$added" ]; then
  x7=1
  while IFS= read -r l; do why "$l"; done <<<"$added"
fi
row X7 "$x7"

x8=0
tests_at() { git grep -cF '#[test]' "$@" -- crates tests 2>/dev/null | awk -F: '{s += $NF} END {print s + 0}'; }
before="$(tests_at "$b")"
after="$(tests_at)"
if [ "$after" -lt "$before" ]; then
  x8=1
  why "#[test] count fell from $before to $after"
fi
deleted="$(git diff --diff-filter=D --name-only "$b" -- web/test)"
if [ -n "$deleted" ]; then
  x8=1
  while IFS= read -r l; do why "deleted: $l"; done <<<"$deleted"
fi
row X8 "$x8"

finish
