---
format: aep.planning-md/3
id: story:draw-ess-068-constructs
kind: story
status: proposed
title: Draw the ESS constructs the page lists as not rendered
summary: 'preserves, system_issued, actor system, undeclared_fields, related_set and at: stamps drawn and held by check'
tags:
- ess
- render
relations:
- depends_on: story:ess-068-crates
scope:
- confidence: cited
  path: crates/ess-ui/src/check.rs
- confidence: cited
  path: crates/ess-ui/src/model.rs
- confidence: cited
  path: crates/ess-ui/src/render.rs
- confidence: cited
  path: crates/ess-ui/tests/pages.rs
- confidence: cited
  path: fixtures/
- confidence: cited
  path: schema/
- confidence: cited
  path: web/src/
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T13:51:07Z", actor: "human:timo", revision: 3}
---
# Story: Draw the ESS constructs the page lists as not rendered

## Outcome

A reader of an `ess-ui` page sees every declaration of a specification drawn, so `ess-ui check`
reports no "not rendered" item for the constructs below.

## Context

On a downstream specification compiled with `ess` 0.68.0, `ess-ui page` (origin/main `8d599a3`)
lists 37 items as not rendered. Grouped from its output:

| construct | items |
|---|---|
| `at:` stamps in authored scenarios not drawn (per scenario) | 13 |
| `system_issued` (command key) | 7 |
| subject effect `preserves` not drawn | 7 |
| `system` (actor key) | 4 |
| `undeclared_fields` (command key) | 3 |
| condition kind `related_set` shown as raw text | 2 |
| `undeclared_fields_ignored` (top-level IR key) | 1 |

Repro (operator machine only, never copied into this repository): `~/.cache/ess-ui-repro/`, its
`README.md` has the commands. Depends on `story:ess-068-crates`.

## Acceptance

- One fixture under `fixtures/`, written fresh with neutral names and compiled with `ess` 0.68.0,
  declares each of the seven constructs above at least once.
- `ess-ui page` on that fixture lists no item as not rendered, and `ess-ui check` on it reports
  `ok   not rendered`.
- `crates/ess-ui/src/check.rs` holds each construct to the IR (a page missing it fails `check`),
  with a test per construct in `crates/ess-ui/tests/pages.rs`.
- `ess-ui check` on `~/.cache/ess-ui-repro` reports 0 not-rendered items.
- `task schema`, `task check` exit 0.

## Out of Scope

- Executing these constructs during replay (`story:check-replay-green`); this story draws them.

## Ambiguities

- How `related_set` is drawn: inferable from how `when_related` is drawn in `render.rs`.
