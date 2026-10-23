# Task: State the ESS 0.68.0 pin in AGENTS.md

## Outcome

`AGENTS.md` § Rules tells the next contributor the pin is ESS 0.68.0 (`ae9d3604…`), and no longer
mentions 0.67.

## Context

- `specification:ess-068-crates` § S3.3.
- `AGENTS.md:24-25` states the pin as ESS 0.67.0 (`4579d804…`).

## Acceptance

- S3.3: `grep -c 'ESS 0.68.0 (`ae9d3604…`)' AGENTS.md` prints `1`.
- S3.3: `grep -c '0\.67' AGENTS.md` prints `0`.
- `grep -c 4579d804 AGENTS.md` prints `0`.

## Scope

- `AGENTS.md:24-25` — cited.

## Out of scope

- Any other rule in `AGENTS.md`.
