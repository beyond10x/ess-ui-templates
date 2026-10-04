# Agents: ess-ui-templates

Tooling repository (a renderer for ESS specifications). It carries no ESS specification of its
own: opt-out recorded here, as the workspace rule asks.

## Layout

| Path | Holds |
|---|---|
| `crates/ess-ui/src/interp.rs` | execution: ESS's `interpret::execute` first, the page's extensions after |
| `crates/ess-ui/src/facts.rs` | fact sources for guards; predicates are decided by ESS's evaluator |
| `crates/ess-ui/src/replay.rs`, `sim.rs` | scenario replay and the baked step scripts |
| `crates/ess-ui/src/model.rs`, `render.rs`, `demo.rs`, `page.rs` | the page, drawn from the IR JSON |
| `crates/ess-ui/src/check.rs` | the page held to the IR, without the rendering code |
| `web/src/` | the browser bundle (JS/CSS); `web/dist/assets.js` is generated from it |
| `web/fonts/` | Inter and Fira Code (latin woff2) and their OFL licences, copied from the docs-system pin by `task fonts`; `web/dist/fonts.js` is generated from them |
| `web/index.js`, `web/compose.js` | the React wrapper and the document it composes |
| `schema/` | the JSON Schema of `ess-ui data` (generated) |
| `fixtures/` | billing (ESS's example), parcel-locker, tally; compiled with `task fixtures` |

## Rules

- Running code is Rust with clap derive. Browser code stays JS/CSS. No Python.
- ESS crates are a git dependency pinned to ESS 0.52.0 (`4d6a4eca…`); fixtures are compiled with
  the same `ess` release. Moving the pin means `task fixtures` and the full `task check`.
- Never write semantics ESS executes. Extend only where `interpret::execute` refuses
  (`NotInterpreted`, `NoValue`) or leaves the outcome open, and add a test in
  `tests/playback.rs` that shows ESS refusing it.
- After editing `web/src/` or the data types, run `task schema`; `task fresh` fails otherwise.
- `web/fonts/` and the tokens block of `web/src/tokens.css` are docs-system's, never edited here:
  moving `DOCS_SYSTEM_PIN` means `task fonts`; `task tokens:check` fails on any difference.
- No source-project names, customer names or hosts anywhere; `task scan` enforces it.

## Extensions over ESS's interpreter

`existing_instance`; creation with a caller-supplied identity; `when_related` with a `holds`
predicate (and its input guard); overlapping related/accepting branches left open, first declared
shown; `when_subject` (`subject_predicate`, `subject_field`); `when_subject_state`, including a
refusal reading its siblings' subject; `sets` from `increment`, related-row fields, caller
attributes, structs and minted timestamps; minted `String` and `Integer` identities; binding
delivery with retry and escalation; views with filter, parameters, aggregation and `order_by`;
the `at` view expectation. Not executed: set effects (`instances`, `affects`), `state_change`,
`input_absent`, retained results and typed responses, current-time guards.

## Commands

`task check` · `task page` · `task screenshots` · `task fixtures` · `task schema`
