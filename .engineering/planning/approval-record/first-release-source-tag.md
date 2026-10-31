---
format: aep.planning-md/3
id: approval-record:first-release-source-tag
kind: approval-record
status: draft
title: First release is a source tag without binaries or a workflow
summary: The orchestrator narrowed story:first-release to a bare-version tag and a GitHub Release
relations:
- decides: story:first-release
revision: 1
---
# Approval: the first release is a source tag

## Approved

b10x-chef-orchestrator, 2026-10-31, release brief D322: "make the first release of ess-ui-templates.
[...] Otherwise make the release a bare-version tag 0.1.0 plus a GitHub Release through the bot,
after check and Shared source gates are green on the tagged commit, and add a Releases section to
AGENTS.md in the release PR. Do not add binaries, a registry publish or a new workflow." Timo,
2026-10-23: "A" (the first release is approved).

## Taken

- `story:first-release` proposed a tag-triggered workflow that builds an x86_64-linux tarball and
  `SHA256SUMS`. The instruction above excludes both for this release, so release 0.1.0 is the
  source tag and the GitHub Release only. The story stays `proposed`: the tarball and the workflow
  are deferred, not dropped.
- The tag is `0.1.0`, from `Cargo.toml` and the story's own ambiguity note. The process is written
  in `AGENTS.md` under Releases.
