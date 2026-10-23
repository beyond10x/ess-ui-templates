#!/usr/bin/env bash
# specification:ess-068-crates § S2 — the lockfile resolves every ESS package to NEW, and only to NEW.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S2' 'Cargo.lock resolves every ESS package to ESS 0.68.0 and nothing else'

declare_row S2.1 'every ESS package source in Cargo.lock is git+https://github.com/beyond10x/ess?rev=NEW#NEW'
declare_row S2.2 'each of the four ESS crates appears in Cargo.lock exactly once'
declare_row S2.3 'the lockfile is current: cargo metadata --locked exits 0'

sources=$(grep '^source = "git+https://github.com/beyond10x/ess' Cargo.lock | sort -u)
[ "$sources" = "source = \"git+https://github.com/beyond10x/ess?rev=$NEW#$NEW\"" ]; row S2.1 $?
[ "$sources" = "source = \"git+https://github.com/beyond10x/ess?rev=$NEW#$NEW\"" ] \
  || why "found: $(printf '%s' "${sources:-<none>}" | tr '\n' ' ')"

bad=()
for c in "${CRATES[@]}"; do
  n=$(grep -c "^name = \"$c\"$" Cargo.lock)
  [ "$n" = 1 ] || bad+=("$c×$n")
done
[ "${#bad[@]}" -eq 0 ]; row S2.2 $?
[ "${#bad[@]}" -eq 0 ] || why "${bad[*]}"

if have cargo; then
  log="$(scratch s2)/metadata.log"
  cargo metadata --locked --format-version 1 > /dev/null 2> "$log"; row S2.3 $?
  [ -s "$log" ] && why "$(tail -n 3 "$log" | tr '\n' ' ')"
else
  row S2.3 1; why 'cargo is not on PATH'
fi

finish
