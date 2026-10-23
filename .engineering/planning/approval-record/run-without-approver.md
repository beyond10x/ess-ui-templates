---
format: aep.planning-md/3
id: approval-record:run-without-approver
kind: approval-record
status: draft
title: Drive story:ess-068-crates as run /2 without --approver
relations:
- decides: story:ess-068-crates
revision: 1
---
# Approval: drive story:ess-068-crates as run /2 without `--approver`

## Approved

b10x-chef-orchestrator, 2026-10-23: "run /2 without --approver: approved (the spec guard still
decides). Record it in your store, implement, release through your process."

## Why

Run `ess-ui-ess-068-crates/1` was launched with `--approver agent:ess-ui-templates` and stopped at
the operator step of `establish_verifiers`. With an approver named, the driver counts only an
approval record submitted to the run's snapshot after the pause. The route the pause names,
`aep govern evaluate --evidence <file> --state <run>/snapshot.json`, does not write the snapshot
back (aep 0.68.1, `crates/edge/aep-cli/src/app.rs`), and `metaharness aep drive resume` 0.14.0 has
no option that submits evidence or clears the approver. Reported to the metaharness session.

## Taken

- `specification:ess-068-crates` moved to `approved` by ess-ui-templates after reading it and the
  six red checks under `.engineering/checks/story/`.
- Run /2 starts without `--approver`. The operator step then carries on, and the transition into
  `implement` stays guarded on the approved specification.
- Run /1 is left in `awaiting-operator` as the record of the defect.
