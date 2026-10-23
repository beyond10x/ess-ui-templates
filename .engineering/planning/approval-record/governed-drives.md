---
format: aep.planning-md/3
id: approval-record:governed-drives
kind: approval-record
status: draft
title: Deliver the four stories as governed metaharness drives
summary: Orchestrator approval and the drive configuration taken inside it
relations:
- decides: story:ess-068-crates
- decides: story:draw-ess-068-constructs
- decides: story:check-replay-green
- decides: story:first-release
revision: 3
---
# Approval: deliver the four stories as governed metaharness drives

## Approved

b10x-chef-orchestrator, 2026-10-23, under the grant in the workspace instructions: "adopt AEP in
this repository …, file one story each for … ; deliver each story as a governed run with
metaharness aep drive run (metaharness 0.14.0, ~/.local/bin), not a plain wave". Timo, 2026-10-23:
"A" and "adopt metaharness".

## Taken inside that approval (ess-ui-templates session, 2026-10-23)

- `protocols:` pinned to AEP `2481b483` (0.68.1), the revision metaharness 0.14.0 builds against;
  its aep cannot read the `aep.driver-steps/2` maps of AEP 0.76.0. Reported to the metaharness
  session.
- Map `development/checks` (Claude Code steps). `development/default` needs a b10x gateway
  endpoint this machine does not name.
- `--allow-evidence-gap`: the checks map declares no step for `contract_result`,
  `property_test_result`, `verification` and `specification`; those records are supplied at the
  pause, before `adversarial_verify -> review`.
- `--pause-on-approval --approver agent:ess-ui-templates`, `--budget-usd 60`,
  `--assume-usd-per-run 5`, one run per story, in dependency order.

## Accepted

b10x-chef-orchestrator, 2026-10-23: "Your AEP 0.68.1 pin and development/checks drive: accepted." and "If the drives run on the Claude Code subscription login, carry on." The drive's claude child runs on the subscription OAuth login, with no API key in its environment (checked on the first run's child process).
