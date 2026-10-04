//! A specification-driven presentation page for ESS: one self-contained HTML page, or its data as
//! JSON, from a compiled ESS model.
//!
//! - [`spec`] compiles a specification with ESS's own compiler.
//! - [`interp`] executes commands: ESS's interpreter first, the page's extensions where ESS does
//!   not execute a construct yet.
//! - [`replay`] and [`sim`] bake every step the page plays.
//! - [`model`], [`render`], [`demo`] and [`page`] draw the page from the IR.
//! - [`check`] holds a page to the IR it was drawn from.
#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::type_complexity
)]

pub mod check;
pub mod demo;
pub mod facts;
pub mod interp;
pub mod model;
pub mod page;
pub mod render;
pub mod replay;
pub mod sim;
pub mod sourceindex;
pub mod spec;
pub mod world;

use std::path::Path;

use anyhow::{bail, Context, Result};

/// Everything one presentation is built from.
pub struct Build {
    /// The compiled IR's text (`ess specify compile --format json`).
    pub ir_text: String,
    /// The parameters.
    pub params: page::Params,
    /// The build inputs.
    pub inputs: page::Inputs,
}

/// Compiles the specification and checks it is the model the IR text was compiled from.
pub fn typed_model(spec_dir: &Path, ir_text: &str) -> Result<ess_compiler::EssIr> {
    let ir =
        spec::compile(spec_dir).with_context(|| format!("compiling {}", spec_dir.display()))?;
    let ours: serde_json::Value = serde_json::from_str(&ir.to_canonical_json())?;
    let theirs: serde_json::Value = serde_json::from_str(ir_text).context("the IR is not JSON")?;
    if ours != theirs {
        bail!(
            "the IR is not the compiled model of {}: compile it again with the ess release this build pins ({})",
            spec_dir.display(),
            ESS_PIN
        );
    }
    Ok(ir)
}

/// The ESS release whose crates this build executes with.
pub const ESS_PIN: &str = "ess 0.52.0 (4d6a4ecafc0feb4e11e4bee777b19c7351fa3647)";

/// Bakes the simulation and builds the presentation.
pub fn present(b: &Build) -> Result<(page::Presentation, sim::Baked)> {
    let baked = match &b.inputs.spec_dir {
        Some(dir) => {
            let ir = typed_model(dir, &b.ir_text)?;
            let interp = interp::Interp::new(&ir);
            let scen_dir = b.inputs.scenario_dir.clone().unwrap_or_else(|| dir.clone());
            sim::bake(
                &interp,
                b.inputs.suite.as_ref().map(|(_, v)| v),
                b.inputs.scenarios.as_ref().map(|(_, v)| v),
                Some(&scen_dir),
                b.params.demo_scenario.as_deref(),
            )
        }
        None => sim::Baked {
            sim: serde_json::json!({"demo": null, "traces": [], "tally": {}, "authored_tally": {}, "failures": [], "authored_failures": [],
                "notes": ["no specification directory was given (`--spec-dir`), so nothing was executed and the Demo has no runs"],
                "metrics": [], "unreached": [], "engine": {"ess": 0, "page": 0, "divergences": []}}),
            authored: std::collections::BTreeMap::new(),
            by: (0, 0),
            divergences: Vec::new(),
        },
    };
    let p = page::build(&b.ir_text, &baked, &b.params, &b.inputs)?;
    Ok((p, baked))
}
