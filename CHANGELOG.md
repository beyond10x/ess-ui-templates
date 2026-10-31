# Changelog

Notable changes to `ess-ui-templates`. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-31

First release, as a source tag. Built against ESS 0.75.0.

### Added

- `ess-ui page` renders a compiled ESS specification as one self-contained HTML page: a demo
  canvas of every entity's lifecycle and a model view of the declarations, with the gate and
  conformance results beside them.
- `ess-ui data`, `ess-ui check`, `ess-ui world`, `ess-ui schema`, `ess-ui assets` and `ess-ui fonts`:
  the presentation as JSON (schema in `schema/`), a check that holds the page to the IR, the final
  view rows of one authored scenario, and the generated browser modules.
- The page executes every scenario and seeded run at build time with ESS's own interpreter and
  plays the baked steps back.
- Inter and Fira Code are embedded, so the page works offline from `file://`.

### Changed

- Design tokens and fonts come from `@beyond10x/docs-system`.
- The demo layout fits documentation-embed widths.
- ESS crates and fixtures follow ESS 0.75.0.

[Unreleased]: https://github.com/beyond10x/ess-ui-templates/compare/0.1.0...HEAD
[0.1.0]: https://github.com/beyond10x/ess-ui-templates/releases/tag/0.1.0
