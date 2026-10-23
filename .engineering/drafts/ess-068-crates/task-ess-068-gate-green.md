# Task: task check and task page pass on the combined ESS 0.68.0 change

Part of `story:ess-068-crates`; covers `specification:ess-068-crates` S3.1 (whole tree), S5.1, S5.4
and S6. Depends on `task:pin-ess-crates`, `task:adapt-ess-ui-to-ess-068`,
`task:regenerate-fixtures-ess-068` and `task:ess-068-text-pins`.

## Scope

- `schema/ess-ui-presentation.schema.json` (cited, `Taskfile.yml:43`; changes only if the 0.68.0
  types change the generated schema)
- `web/dist/assets.js`, `web/dist/fonts.js` (cited, `Taskfile.yml:44-45`; regenerated only if stale)

## Work

On the tree with all four sibling tasks applied, run `task schema`. If `schema/` or `web/dist/`
changed, commit them. Edit nothing that a sibling task owns. A failure that a sibling's surface would
fix goes back to that sibling's task.

## Acceptance

On the combined tree, the repository's whole gate and the page build pass, and no file outside
`.engineering/` still names ESS 0.67.0. Each check below must hold:

- S5.1 `task check` exits 0.
- S5.4 `task fresh` exits 0. It is part of `task check`, and it is listed so a verifier can run it on its own.
- S6.1 `task page` exits 0, then `test -s out/billing.html && test -s out/parcel-locker.html && test -s out/billing.data.json` exits 0.
- S6.2 `task web:test` exits 0 and `cargo test --locked` exits 0. Both are in `task check`, and they
  are listed so a verifier can run them on their own.
- S3.1 `git grep -n -e 4579d8049bb5b83749b46059dfc0982de7633485 -e 4579d804 -- ':!.engineering'` prints nothing.
