//! The page drawn from ESS's billing example, the parcel-locker fixture and the tally fixture: every
//! declaration on the page, every source line real, every scenario replayed, the data valid.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ess_ui::page::{Inputs, Params, Presentation};
use ess_ui::Build;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn json(p: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
    )
    .unwrap()
}

fn build(name: &str) -> Build {
    let d = root().join("fixtures").join(name);
    let scen = if d.join("scenarios").is_dir() {
        d.join("scenarios")
    } else {
        d.join("spec")
    };
    Build {
        ir_text: std::fs::read_to_string(d.join("ir.json")).unwrap(),
        params: Params {
            git_ref: Some("fixture".into()),
            ..Params::default()
        },
        inputs: Inputs {
            spec_dir: Some(d.join("spec")),
            scenario_dir: Some(scen),
            suite: Some(("suite.json".into(), json(&d.join("suite.json")))),
            scenarios: Some(("authored.json".into(), json(&d.join("authored.json")))),
            out: Some(root().join("out").join(format!("{name}.html"))),
            ..Inputs::default()
        },
    }
}

fn page(name: &str) -> (Presentation, String, ess_ui::sim::Baked, Build) {
    let b = build(name);
    let (p, baked) = ess_ui::present(&b).unwrap();
    let html = ess_ui::page::html(&p);
    (p, html, baked, b)
}

fn every_check_passes(name: &str) {
    let (p, html, baked, b) = page(name);
    let mut rep = ess_ui::check::check(&b.ir_text, &html, b.inputs.spec_dir.as_deref());
    ess_ui::check::check_simulation(&mut rep, &html, &baked);
    assert!(rep.passed(), "{name}:\n{}", rep.render());
    assert!(
        p.unrendered.is_empty(),
        "{name}: not rendered: {:?}",
        p.unrendered
    );

    // Every declaration of the IR, of every kind, is a declaration the page knows by name.
    let ir: Value = serde_json::from_str(&b.ir_text).unwrap();
    let known: BTreeSet<(String, String)> = p.model["decls"]
        .as_object()
        .unwrap()
        .values()
        .map(|d| {
            (
                d["k"].as_str().unwrap().to_owned(),
                d["i"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    for (kind, coll) in [
        ("domain", "domains"),
        ("component", "components"),
        ("actor", "actors"),
        ("entity", "entities"),
        ("command", "commands"),
        ("event", "events"),
        ("view", "views"),
        ("error", "errors"),
        ("type", "types"),
        ("binding", "bindings"),
    ] {
        for n in ir[coll].as_object().into_iter().flatten().map(|(k, _)| k) {
            assert!(
                known.contains(&(kind.to_owned(), n.clone())),
                "{name}: {kind} `{n}` is not on the page"
            );
        }
    }
}

#[test]
fn billing_page_draws_every_declaration_and_passes_every_check() {
    every_check_passes("billing");
}

#[test]
fn parcel_locker_page_draws_every_declaration_and_passes_every_check() {
    every_check_passes("parcel-locker");
}

#[test]
fn tally_page_draws_every_declaration_and_passes_every_check() {
    every_check_passes("tally");
}

#[test]
fn billing_runs_entirely_on_ess_interpreter() {
    let (_, _, baked, _) = page("billing");
    assert!(baked.by.0 > 0);
    assert_eq!(baked.by.1, 0, "every billing command is one ESS executes");
    assert!(baked.divergences.is_empty(), "{:?}", baked.divergences);
}

#[test]
fn the_same_inputs_give_the_same_bytes() {
    let (_, a, _, _) = page("parcel-locker");
    let (_, b, _, _) = page("parcel-locker");
    assert!(a == b, "two builds of one page differ");
}

#[test]
fn the_data_is_valid_against_the_committed_schema_and_the_schema_is_current() {
    let committed = json(&root().join("schema/ess-ui-presentation.schema.json"));
    let generated = serde_json::to_value(schemars::schema_for!(Presentation)).unwrap();
    assert_eq!(committed, generated, "schema/ is stale: run `task schema`");
    let validator = jsonschema::validator_for(&committed).unwrap();
    for name in ["billing", "parcel-locker", "tally"] {
        let (p, ..) = page(name);
        let data = serde_json::to_value(&p).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&data)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        assert_eq!(data["format"], "ess-ui-presentation/1");
    }
}

#[test]
fn the_web_bundle_module_is_current() {
    let committed = std::fs::read_to_string(root().join("web/dist/assets.js")).unwrap();
    for part in ess_ui::page::assets::CSS
        .iter()
        .chain(ess_ui::page::assets::JS.iter())
    {
        let encoded = serde_json::to_string(part).unwrap();
        let encoded = &encoded[1..encoded.len() - 1];
        assert!(
            committed.contains(encoded),
            "web/dist/assets.js is stale: run `task schema`"
        );
    }
}
