//! Loads and compiles an ESS specification through ESS's own parser, assembler and compiler.
//!
//! The file selection follows `ess`: the `specification:` list of `ess-inputs.yaml` when the
//! directory has one, otherwise every `.yaml`/`.yml` file under a directory that holds
//! `system.yaml`, in path order. The compiled model's canonical JSON is byte-identical to
//! `ess specify compile --format json`, which is how a page checks that the IR it was handed and
//! the specification it executes are one model.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use ess_compiler::source::SourceMap;
use ess_compiler::EssIr;

/// The files a specification is made of, relative to its directory.
pub fn spec_files(dir: &Path) -> Result<Vec<String>> {
    let manifest = dir.join("ess-inputs.yaml");
    if manifest.is_file() {
        let text = fs::read_to_string(&manifest)
            .with_context(|| format!("reading {}", manifest.display()))?;
        let doc: serde_yaml::Value = serde_yaml::from_str(&text)
            .with_context(|| format!("parsing {}", manifest.display()))?;
        let list = doc
            .get("specification")
            .and_then(serde_yaml::Value::as_sequence)
            .map(|s| {
                s.iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        return Ok(list);
    }
    if !dir.join("system.yaml").is_file() {
        bail!(
            "{} is not an ESS specification: it holds no `system.yaml` and no `ess-inputs.yaml`",
            dir.display()
        );
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    let mut children: Vec<PathBuf> = fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<_>>()?;
    children.sort();
    for child in children {
        if child.is_dir() {
            walk(root, &child, out)?;
        } else if child.extension().is_some_and(|x| x == "yaml" || x == "yml") {
            let rel = child
                .strip_prefix(root)
                .unwrap_or(&child)
                .to_string_lossy()
                .replace('\\', "/");
            if rel != "ess-inputs.yaml" {
                out.push(rel);
            }
        }
    }
    Ok(())
}

/// The compiled model of the specification in `dir`.
pub fn compile(dir: &Path) -> Result<EssIr> {
    let files = spec_files(dir)?;
    let mut texts = Vec::new();
    for rel in &files {
        let path = dir.join(rel);
        texts.push((
            rel.clone(),
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?,
        ));
    }
    let every: Vec<&str> = texts.iter().map(|(_, t)| t.as_str()).collect();
    let results = ess_domain::spec::RawSpecFile::parse_all(&every);
    let mut parsed = Vec::new();
    let mut map = SourceMap::new();
    let mut problems = Vec::new();
    for ((rel, text), result) in texts.iter().zip(results) {
        let source = ess_domain::system::Source::new(rel.clone());
        map.insert(source.as_str(), text.as_str());
        match result {
            Ok(raw) => parsed.push((source, raw)),
            Err(error) => problems.push(format!("{rel}: {error}")),
        }
    }
    if !problems.is_empty() {
        bail!("{} does not parse:\n{}", dir.display(), problems.join("\n"));
    }
    let assembled = ess_domain::spec::Specification::assemble(parsed).map_err(|errors| {
        anyhow::anyhow!(
            "{} does not assemble: {}",
            dir.display(),
            errors
                .as_slice()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    ess_compiler::compile(&assembled, &map)
        .map_err(|d| anyhow::anyhow!("{} does not compile:\n{d}", dir.display()))
}
