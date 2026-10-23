---
format: aep.planning-md/3
id: story:first-release
kind: story
status: proposed
title: 'First ess-ui release: x86_64-linux tarball and SHA256SUMS'
summary: A tag-triggered release workflow publishes the binary and its checksums through the bot route
tags:
- release
relations:
- depends_on: story:ess-068-crates
- depends_on: story:draw-ess-068-constructs
- depends_on: story:check-replay-green
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-23T13:51:07Z", actor: "human:timo", revision: 3}
---
# Story: First `ess-ui` release, x86_64-linux tarball and SHA256SUMS

## Outcome

A specification author downloads `ess-ui` for x86_64 Linux from a GitHub Release and checks it
against `SHA256SUMS`, instead of building from source.

## Context

- The repository has no release workflow (`.github/workflows/` holds `check.yml` and
  `shared-gates.yml`), no `CHANGELOG.md`, and `Cargo.toml:6` says `version = "0.1.0"`.
- Releases go through the bot route (`~/beyond10x/AGENTS.md` § GitHub writes are the bot's) and are
  finished only once the tag, its checks, the GitHub Release and its artifacts are verified.
- Ships after `story:ess-068-crates`, `story:draw-ess-068-constructs` and `story:check-replay-green`.

## Acceptance

- `.github/workflows/release.yml` builds `ess-ui` on a tag push for `x86_64-unknown-linux-gnu`,
  packs `ess-ui-<tag>-x86_64-unknown-linux-gnu.tar.gz` (binary, `LICENSE`, `README.md`), writes
  `SHA256SUMS`, and attaches both to the GitHub Release for that tag. Actions are pinned by commit.
- `CHANGELOG.md` in Keep a Changelog 1.1.0 with the release's section.
- The tag is pushed by `b10x-bot[bot]`; the release workflow and the shared gates on the tag succeed.
- `sha256sum -c SHA256SUMS` passes on the downloaded tarball, and the unpacked `ess-ui --version`
  prints the tag's version.
- `verify-release.sh ess-ui-templates <tag>` exits 0.

## Out of Scope

- Other targets (macOS, aarch64), crates.io publishing, an installer script.

## Ambiguities

- The first tag's version: inferable, `0.1.0` from `Cargo.toml:6`, unless a story above changes it.
