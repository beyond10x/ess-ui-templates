#!/usr/bin/env bash
# specification:ess-068-crates § S1 — the manifest pins every ESS crate to NEW.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S1' 'Cargo.toml pins every ESS crate to ESS 0.68.0'

declare_row S1.1 'each of the four ESS crates has one workspace line with git = beyond10x/ess and rev = NEW'
declare_row S1.2 'no ESS dependency in Cargo.toml uses branch, tag or a rev other than NEW'
declare_row S1.3 'no crate manifest under crates/ pins ESS of its own'

n=$(grep -cE '^(ess-primitives|ess-domain|ess-compiler|ess-conformance) = \{ git = "https://github.com/beyond10x/ess", rev = "'"$NEW"'" \}$' Cargo.toml)
[ "$n" = 4 ]; row S1.1 $?
[ "$n" = 4 ] || why "$n of 4 crates pinned to $NEW"

other=$(grep -n 'beyond10x/ess"' Cargo.toml | grep -v 'rev = "'"$NEW"'"')
# Vacuous when Cargo.toml names ESS nowhere: S1.1 is what keeps that red.
[ -z "$other" ]; row S1.2 $?
[ -z "$other" ] || why "$(printf '%s' "$other" | head -n 4 | tr '\n' ' ')"

own=$(grep -rn 'beyond10x/ess' crates/*/Cargo.toml 2>/dev/null)
[ -z "$own" ]; row S1.3 $?
[ -z "$own" ] || why "$(printf '%s' "$own" | head -n 4 | tr '\n' ' ')"

finish
