---
format: aep.planning-md/3
id: story:ess-068-crates
kind: story
status: implemented
title: Move the ESS crates and fixtures to ESS 0.68.0
summary: Pin the four ESS crates and the fixture compiler to ESS 0.68.0 and keep task check green
tags:
- dependency
- ess
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: fixtures/
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T13:51:07Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-23T20:04:35Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-23T20:05:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
# Story: Move the ESS crates and fixtures to ESS 0.68.0

## Outcome

A specification author on ESS 0.68.0 gets a page from `ess-ui` that reads the IR, suites and
authored scenarios their `ess` 0.68.0 wrote, because the renderer links the same ESS release.

## Context

- `Cargo.toml:14-17` pins `ess-primitives`, `ess-domain`, `ess-compiler` and `ess-conformance` to
  ESS 0.67.0 (`4579d804…`); `AGENTS.md` § Rules states the same pin.
- ESS 0.68.0 is tag `0.68.0`, commit `ae9d3604ec001b3f0f318e8d961748c388082f60`
  (`git ls-remote https://github.com/beyond10x/ess refs/tags/0.68.0^{}`).
- The workspace rule is the newest ESS release everywhere (`~/beyond10x/AGENTS.md` § ESS version).
- A downstream specification compiled with `ess` 0.68.0 replays with 822 failed synthesized steps,
  many of them `cannot parse predicate "": expression is empty`; part of that may be the 0.67/0.68
  skew this story removes. `story:check-replay-green` owns what remains.

## Acceptance

- `Cargo.toml` pins all four ESS crates to `ae9d3604ec001b3f0f318e8d961748c388082f60`, and
  `Cargo.lock` resolves them to that commit.
- `task fixtures`, run with an `ess` 0.68.0 binary, regenerates `fixtures/*/{ir,suite,authored}.json`
  and `synthesize.log`; the diff is committed.
- `Taskfile.yml` `BILLING_REPO` `--ref` names the 0.68.0 commit.
- `AGENTS.md` names ESS 0.68.0 and `ae9d3604…`.
- `task check` exits 0.

## Out of Scope

- Drawing constructs the page lists as not rendered (`story:draw-ess-068-constructs`).
- Replay failures that remain after the move (`story:check-replay-green`).

## Ambiguities

None.
