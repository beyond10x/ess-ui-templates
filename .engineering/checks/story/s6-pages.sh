#!/usr/bin/env bash
# specification:ess-068-crates § S6 — the pages build from the regenerated fixtures.
#
# Runs only on the new pin, for the reason s5-gate.sh gives: the pages build on 0.67.0 today.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S6' 'task page builds the billing and parcel-locker pages from the 0.68.0 fixtures on the 0.68.0 crates'

declare_row S6.1 'task page exits 0 and writes non-empty out/billing.html, out/parcel-locker.html, out/billing.data.json'
declare_row S6.2 'out/billing.data.json satisfies the schema (task web:test composes the page from it)'

if ! have task; then
  red_all 'task is not on PATH'
elif ! { pinned_to_new && locked_to_new; }; then
  red_all 'Cargo.toml/Cargo.lock are not on NEW (S1.1, S2.1) — pages built on the old pin decide nothing'
else
  work=$(scratch s6)
  rm -f out/billing.html out/parcel-locker.html out/billing.data.json
  ok=1
  task page > "$work/page.log" 2>&1 \
    && test -s out/billing.html && test -s out/parcel-locker.html && test -s out/billing.data.json \
    && ok=0
  row S6.1 "$ok"
  [ "$ok" -eq 0 ] || why "log: ${work#"$REPO"/}/page.log"

  if [ "$ok" -ne 0 ]; then
    row S6.2 1; why 'no data.json from S6.1 to validate'
  else
    task web:test > "$work/web-test.log" 2>&1; row S6.2 $?
    why "log: ${work#"$REPO"/}/web-test.log"
  fi
fi

finish
