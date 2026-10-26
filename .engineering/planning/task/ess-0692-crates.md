---
format: aep.planning-md/3
id: task:ess-0692-crates
kind: task
status: implemented
title: Move the ESS crates and fixtures to ESS 0.69.2
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-26T16:12:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-26T16:12:26Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-26T16:12:26Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"verification":1}}}
---
# Task: move the ESS crates and fixtures to ESS 0.69.2

## Context

The workspace rule is to use the newest ESS release everywhere (operator, 2026-10-05: "ALWAYS
upgrade to most recent ESS"). Session b10x-chef-orchestrator approved this move on 2026-10-26 as a
class-C dependency upgrade. The previous move, to 0.68.0, is `story:ess-068-crates` (commit
`fe7be68`).

## Acceptance

`ess-primitives`, `ess-domain`, `ess-compiler` and `ess-conformance` are pinned to `a3bc2df3`
(tag 0.69.2) in `Cargo.toml` and `Cargo.lock`; `ESS_PIN`, the billing `--ref` in `Taskfile.yml`,
AGENTS.md, README.md and the workaround notes in `interp.rs` and `tests/playback.rs` name 0.69.2.
`task fixtures` with ess 0.69.2 leaves `ir.json`, `suite.json` and `authored.json` byte-identical
(only the version line of `parcel-locker/synthesize.log` changes); `examples/billing` and
`examples/billing-scenarios` are unchanged between the two ESS tags. `task check` exits 0.
