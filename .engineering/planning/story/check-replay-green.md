---
format: aep.planning-md/3
id: story:check-replay-green
kind: story
status: proposed
title: ess-ui check replays a downstream ESS 0.68.0 specification green
summary: Synthesized and authored replay and the ESS lockstep pass, or fail on a reported ESS gap
tags:
- ess
- replay
relations:
- depends_on: story:ess-068-crates
scope:
- confidence: cited
  path: crates/ess-ui/src/facts.rs
- confidence: cited
  path: crates/ess-ui/src/interp.rs
- confidence: cited
  path: crates/ess-ui/src/replay.rs
- confidence: cited
  path: crates/ess-ui/src/sim.rs
- confidence: cited
  path: crates/ess-ui/tests/playback.rs
- confidence: cited
  path: fixtures/
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T13:51:07Z", actor: "human:timo", revision: 3}
---
# Story: `ess-ui check` replays a downstream ESS 0.68.0 specification green

## Outcome

A specification author running `ess-ui check` on a 0.68.0 specification gets PASS on the
synthesized replay, the authored replay and the ESS lockstep, or a refusal that names an ESS gap.

## Context

On a downstream specification compiled with `ess` 0.68.0 (213 synthesized, 27 refusals, 14
authored), `ess-ui check` at `8d599a3` fails:

| check | result |
|---|---|
| replay synthesized suite | 822 failed steps; first classes: `expect_view: undetermined: cannot parse predicate "": expression is empty`, `capture_instance: … = None`, `expect_outcome: got started` |
| replay authored scenarios | 92 failed steps; first classes: `expect_outcome ready: got registered-fetched`, `expect_event … not emitted`, `expect_view … not found` |
| ESS lockstep | 1 divergence: ESS answers `already-initialized`, the page cannot: "the identity `session_id` is already held" |

Repro: `~/.cache/ess-ui-repro/` (operator machine only, never copied here). Depends on
`story:ess-068-crates`, whose crate move may clear part of the 822.

## Acceptance

- Each failure class is classified in this story's evidence as an `ess-ui` defect or an ESS gap;
  each ESS gap is reported to the ess session with ESS version, `file:line` and a repro path.
- Each `ess-ui` defect has a failing test in `crates/ess-ui/tests/playback.rs` on a fresh neutral
  fixture before its fix, passing after.
- `ess-ui check` on `~/.cache/ess-ui-repro` reports `ok` for replay synthesized, replay authored and
  ESS lockstep, except steps attributed to a reported ESS gap, which are listed by id.
- No semantics ESS executes are written here (`AGENTS.md` § Rules); each extension carries a test
  that shows ESS refusing it.
- `task check` exits 0.

## Out of Scope

- Drawing not-rendered constructs (`story:draw-ess-068-constructs`).
- Fixing ESS itself.

## Ambiguities

- Whether the empty-predicate `expect_view` steps are ESS emitting an empty predicate or `ess-ui`
  reading a moved field: inferable by diffing the 0.67.0 and 0.68.0 suite JSON for one step.
