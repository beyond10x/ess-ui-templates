//! A sample world: one authored scenario replayed, and the final rows of every view written out as
//! view fixtures (the `rows` list of an ess-ui view fixture), one JSON array per view.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{bail, Result};
use serde_json::Value;

use crate::interp::Interp;
use crate::replay::{failures, jfields, replay};

/// A number with no fractional part written as an integer, as the scenario wrote it.
fn plain(v: Value) -> Value {
    match v {
        Value::Number(n) => match n.as_f64() {
            #[allow(clippy::cast_possible_truncation)]
            Some(f) if f.fract() == 0.0 && f.abs() < 9.0e15 && !n.is_i64() && !n.is_u64() => {
                Value::from(f as i64)
            }
            _ => Value::Number(n),
        },
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, x)| (k, plain(x))).collect()),
        Value::Array(a) => Value::Array(a.into_iter().map(plain).collect()),
        other => other,
    }
}

/// `{view: rows}` after replaying `scenario`, and the steps that failed.
pub fn world(
    interp: &Interp<'_>,
    scenario: &Value,
    name: &str,
) -> (BTreeMap<String, Vec<Value>>, Vec<String>) {
    let (trace, engine) = replay(interp, scenario, true);
    let failed = failures(&BTreeMap::from([(name.to_owned(), trace)]));
    let views = interp
        .ir
        .views()
        .keys()
        .map(|v| {
            let rows = interp
                .view_rows(&engine.world, &v.to_string(), None)
                .unwrap_or_default();
            (
                v.to_string(),
                rows.iter().map(|(_, r)| plain(jfields(r))).collect(),
            )
        })
        .collect();
    (views, failed)
}

/// Writes one `<view>.json` per view and a `README.md` into `out`, replacing it.
pub fn write(interp: &Interp<'_>, suite: &Value, scenario: &str, out: &Path) -> Result<String> {
    let scenarios = suite
        .get("scenarios")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let keys: Vec<&String> = scenarios
        .keys()
        .filter(|k| *k == scenario || k.ends_with(&format!("/authored/{scenario}")))
        .collect();
    let [key] = keys.as_slice() else {
        bail!("no single authored scenario `{scenario}` in the scenarios given")
    };
    let (views, failed) = world(interp, &scenarios[*key], scenario);
    if !failed.is_empty() {
        bail!(
            "the replay of `{scenario}` failed at {} step(s):\n  {}",
            failed.len(),
            failed.join("\n  ")
        );
    }
    if out.exists() {
        std::fs::remove_dir_all(out)?;
    }
    std::fs::create_dir_all(out)?;
    for (name, rows) in &views {
        std::fs::write(
            out.join(format!("{name}.json")),
            serde_json::to_string_pretty(rows)? + "\n",
        )?;
    }
    let empty: Vec<&String> = views
        .iter()
        .filter(|(_, r)| r.is_empty())
        .map(|(n, _)| n)
        .collect();
    let mut md = format!(
        "# Sample world: `{scenario}`\n\nOne replay of the authored scenario `{scenario}` through `ess-ui world`. Each `<view>.json` is the JSON array of that view's rows at the end of the replay: the `rows` list of an ess-ui view fixture. Identities agree across views: they come from one run.\n\n{} views, {} with rows, {} empty.\n\n## Views with parameters\n\nA view that declares parameters is written unfiltered: every row, read with no parameter bound.\n\n",
        views.len(),
        views.len() - empty.len(),
        empty.len()
    );
    let with_params: Vec<(String, Vec<String>)> = interp
        .ir
        .views()
        .iter()
        .filter(|(_, v)| !v.params.is_empty())
        .map(|(n, v)| {
            (
                n.to_string(),
                v.params.iter().map(|p| p.name.clone()).collect(),
            )
        })
        .collect();
    if with_params.is_empty() {
        md.push_str("None.\n");
    }
    for (n, ps) in &with_params {
        let _ = writeln!(
            md,
            "- `{n}`: {}",
            ps.iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    md.push_str("\n## Empty views\n\n");
    if empty.is_empty() {
        md.push_str("None.\n");
    }
    for n in &empty {
        let _ = writeln!(md, "- `{n}`");
    }
    md.push_str("\n## Rows per view\n\n| View | Rows |\n|---|---|\n");
    for (n, rows) in &views {
        let _ = writeln!(md, "| `{n}` | {} |", rows.len());
    }
    std::fs::write(out.join("README.md"), md)?;
    Ok(format!(
        "{} view(s) written to {}, {} with rows, {} empty",
        views.len(),
        out.display(),
        views.len() - empty.len(),
        empty.len()
    ))
}
