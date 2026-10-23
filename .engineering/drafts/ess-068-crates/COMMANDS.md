# Store writes still owed for the ess-068-crates decomposition

These were drafted in workflow state `decompose`. That state has no shell, so none of the writes below
has run. Nothing under `.engineering/planning/` was created or edited.

Before running them, confirm the vocabulary, because these assumptions were not checked:

    aep plan artifact kinds              # is `task` a kind?
    aep plan artifact relations          # are `decomposes` and `depends_on` the edge names, and may a task decompose a story?

Then, one writer at a time:

    D=.engineering/drafts/ess-068-crates
    aep plan artifact new task pin-ess-crates \
      --title "Pin the four ESS crates to ESS 0.68.0 in Cargo.toml and Cargo.lock" \
      --relate decomposes:story:ess-068-crates --from $D/task-pin-ess-crates.md
    aep plan artifact new task regenerate-fixtures-ess-068 \
      --title "Regenerate the fixtures with the ess 0.68.0 CLI and commit them" \
      --relate decomposes:story:ess-068-crates --from $D/task-regenerate-fixtures-ess-068.md
    aep plan artifact new task ess-068-text-pins \
      --title "Name ESS 0.68.0 in the Taskfile's BILLING_REPO and in AGENTS.md" \
      --relate decomposes:story:ess-068-crates --from $D/task-ess-068-text-pins.md
    aep plan artifact new task adapt-ess-ui-to-ess-068 \
      --title "Make ess-ui compile, lint clean and pass its tests against the ESS 0.68.0 crates" \
      --relate decomposes:story:ess-068-crates --from $D/task-adapt-ess-ui-to-ess-068.md
    aep plan artifact relate task:adapt-ess-ui-to-ess-068 depends_on task:pin-ess-crates
    aep plan artifact new task ess-068-gate-green \
      --title "task check and task page pass on the combined ESS 0.68.0 change" \
      --relate decomposes:story:ess-068-crates --from $D/task-ess-068-gate-green.md
    for t in pin-ess-crates adapt-ess-ui-to-ess-068 regenerate-fixtures-ess-068 ess-068-text-pins; do
      aep plan artifact relate task:ess-068-gate-green depends_on task:$t
    done
    aep plan artifact validate

Shape: `pin-ess-crates`, `regenerate-fixtures-ess-068` and `ess-068-text-pins` touch separate files
and can run in parallel. `adapt-ess-ui-to-ess-068` follows `pin-ess-crates`. `ess-068-gate-green`
comes last.

`.engineering/drafts/` is not in `.gitignore`, so keep these files out of any commit, or add the
directory to `.gitignore`.
