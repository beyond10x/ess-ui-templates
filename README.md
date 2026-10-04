# ess-ui-templates

A presentation page for any [ESS](https://github.com/beyond10x/ess) specification: one
self-contained HTML page, generated from the compiled model, that works offline from `file://`.

- **Demo**: every entity's lifecycle on one canvas, grouped by owning component. Instance tokens
  move through their state machines while a scenario or a seeded run plays; actors on the left;
  views, events, a sequence diagram, checks and an instance inspector on the right; swimlanes
  along the bottom; playback controls, scrubber and speed.
- **Model**: overview and gate results, architecture, every lifecycle with its path player,
  simulation metrics, guarded branches, entities and relations, commands, events, views,
  bindings, errors and types, a source index linking every declaration to its line, and the
  change view.

Keyboard control, a command palette (Ctrl/Cmd+K), presentation mode (`p`), deep links, URL state
and light and dark themes come with it. The look is the beyond10x product sites' design tokens
(`@beyond10x/docs-system`, pinned in `package.json`).

## Use it

```sh
ess specify compile --path spec --format json --out build/ir.json
ess verify conform synthesize --path spec --out build/suite.json
ess verify conform author --path spec --scenarios scenarios --out build/authored.json

ess-ui page --ir build/ir.json --spec-dir spec \
  --suite build/suite.json --scenarios build/authored.json --scenario-dir scenarios \
  --repo-url https://github.com/<owner>/<repo> --spec-root spec \
  --out build/page.html
```

`--spec-dir` is what lets the page execute the model: it is compiled with ESS's own compiler, must
be the model the IR was compiled from, and every scenario and seeded run is executed at build time.
The page only plays the baked steps back.

| Command | Writes |
|---|---|
| `ess-ui page` | the self-contained HTML page |
| `ess-ui data` | the same presentation as JSON (`ess-ui-presentation/1`, schema in `schema/`) |
| `ess-ui check --html <page>` | nothing; exits 1 unless the page draws exactly what the IR declares |
| `ess-ui world --scenario <name> --out <dir>` | the final rows of every view after one authored scenario |
| `ess-ui schema`, `ess-ui assets` | the JSON Schema, and the browser bundle as an ES module |

Parameters can also come from `<spec-dir>/ess-ui.json` (`title`, `repo_url`, `repo_label`, `ref`,
`spec_root`, `source_url`, `tree_url`, `headline_entity`, `section_order`, `demo`, `present`,
`demo_scenario`); flags override the file. Source links default to
`{repo}/blob/{ref}/{path}#L{line}`; set `source_url` and `tree_url` for another host.

## In a Docusaurus product site

```js
import { EssPresentation } from '@beyond10x/ess-ui-templates';
import data from './billing.presentation.json'; // written by `ess-ui data`

<EssPresentation data={data} />
```

Add the package by commit (`"@beyond10x/ess-ui-templates": "github:beyond10x/ess-ui-templates#<sha>"`,
path `web/`). The component renders the page in a sandboxed frame, so its keyboard handling and
styles stay its own, and it follows the site's light or dark theme.

## How the model is executed

ESS's interpreter (`ess-conformance`, `interpret::execute`) answers every step it determines.
Where it does not execute a construct yet, the page's extension of it answers, and while both can
read a step they are compared; a difference fails `ess-ui check`. See `AGENTS.md` for the list of
extensions.

## Develop

`task check` runs fmt, clippy, the tests, the schema and bundle freshness checks, the React
wrapper's composition test and the name scan. `task fixtures` recompiles the fixtures with the
`ess` CLI; `task screenshots` writes headless-Chrome screenshots of the billing page.

`fixtures/billing` is ESS's own `examples/billing` and `examples/billing-scenarios` at ESS 0.52.0.

Licensed under Apache-2.0. Ported from an earlier internal prototype.
