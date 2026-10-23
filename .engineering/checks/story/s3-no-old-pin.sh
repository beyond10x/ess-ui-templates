#!/usr/bin/env bash
# specification:ess-068-crates § S3 — nothing in the tracked tree still names ESS 0.67.0.
. "$(dirname "${BASH_SOURCE[0]}")/../lib.sh"

unit 'specification:ess-068-crates#S3' 'no tracked file outside .engineering names ESS 0.67.0; Taskfile.yml and AGENTS.md name 0.68.0'

declare_row S3.1 'git grep finds OLD (full or 4579d804) nowhere outside .engineering'
declare_row S3.2 'Taskfile.yml BILLING_REPO ends --ref NEW'
declare_row S3.3 'AGENTS.md states the pin as ESS 0.68.0 (ae9d3604…) and names 0.67 nowhere'

hits=$(git grep -n -e "$OLD" -e "${OLD:0:8}" -- ':!.engineering')
[ -z "$hits" ]; row S3.1 $?
[ -z "$hits" ] || why "$(printf '%s\n' "$hits" | cut -d: -f1,2 | head -n 6 | tr '\n' ' ')"

n=$(grep -c "^  BILLING_REPO: .* --ref $NEW\$" Taskfile.yml)
[ "$n" = 1 ]; row S3.2 $?
[ "$n" = 1 ] || why "$(grep '^  BILLING_REPO:' Taskfile.yml | sed 's/.*--ref/--ref/')"

new=$(grep -c 'ESS 0.68.0 (`ae9d3604…`)' AGENTS.md)
old=$(grep -c '0\.67' AGENTS.md)
[ "$new" = 1 ] && [ "$old" = 0 ]; row S3.3 $?
{ [ "$new" = 1 ] && [ "$old" = 0 ]; } || why "0.68.0 pin lines: $new (want 1); 0.67 mentions: $old (want 0)"

finish
