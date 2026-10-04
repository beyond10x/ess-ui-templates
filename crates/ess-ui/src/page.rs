//! The presentation: its data (the JSON a React wrapper renders) and the self-contained page.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::demo::{
    diff_index, parse_gate, render_changes, render_demo, render_overview, render_simulation,
};
use crate::model::{
    arr, assign_text, condition_text, display, g, predicate_text, short, st, strs, summary, text,
    type_label, Model, Unrendered,
};
use crate::render::{
    domain_legend, esc, legend_html, live_panel, render_architecture, render_branches,
    render_commands, render_er, render_errors_types, render_events, render_interactions,
    render_lifecycle, render_source_index, render_views, slug, src_link, tag,
};
use crate::sim::Baked;
use crate::sourceindex::SourceIndex;

/// The data format this crate writes.
pub const FORMAT: &str = "ess-ui-presentation/1";

/// Section ids, in their default order.
pub const DEFAULT_ORDER: [&str; 15] = [
    "overview",
    "arch",
    "lifecycle",
    "simulation",
    "branches",
    "entities",
    "commands",
    "events",
    "views",
    "interactions",
    "errors",
    "sources",
    "changes",
    "legend",
    "coverage",
];

/// The browser bundle, embedded at build time.
pub mod assets {
    /// The stylesheet: design tokens, the page, the demo, and the product layer.
    pub const CSS: [&str; 4] = [
        include_str!("../../../web/src/tokens.css"),
        include_str!("../../../web/src/page.css"),
        include_str!("../../../web/src/demo.css"),
        include_str!("../../../web/src/product.css"),
    ];
    /// The scripts, in load order.
    pub const JS: [&str; 3] = [
        include_str!("../../../web/src/page.js"),
        include_str!("../../../web/src/demo.js"),
        include_str!("../../../web/src/ui.js"),
    ];
    /// The script that sets the theme before the page paints.
    pub const HEAD_JS: &str = include_str!("../../../web/src/head.js");
    /// The SVG markers every diagram uses.
    pub const DEFS: &str = include_str!("../../../web/src/defs.svg");

    /// Every stylesheet, concatenated.
    pub fn css() -> String {
        CSS.concat()
    }

    /// Every script, concatenated.
    pub fn js() -> String {
        JS.join("\n")
    }

    /// One self-hosted font file: family, weight range and the woff2 bytes.
    pub struct Font {
        pub family: &'static str,
        pub file: &'static str,
        pub weight: &'static str,
        pub woff2: &'static [u8],
    }

    /// The latin-subset fonts `@beyond10x/docs-system` self-hosts under `b10x-fonts/`, copied from
    /// the pinned commit into `web/fonts/` by `task fonts`; `task tokens:check` compares them.
    pub const FONTS: [Font; 3] = [
        Font {
            family: "Inter",
            file: "inter-variable-latin.woff2",
            weight: "100 900",
            woff2: include_bytes!("../../../web/fonts/inter-variable-latin.woff2"),
        },
        Font {
            family: "Fira Code",
            file: "fira-code-regular-latin.woff2",
            weight: "400",
            woff2: include_bytes!("../../../web/fonts/fira-code-regular-latin.woff2"),
        },
        Font {
            family: "Fira Code",
            file: "fira-code-semibold-latin.woff2",
            weight: "600",
            woff2: include_bytes!("../../../web/fonts/fira-code-semibold-latin.woff2"),
        },
    ];

    /// The OFL licence of each family, embedded beside its faces.
    pub const LICENCES: [(&str, &str); 2] = [
        ("Inter", include_str!("../../../web/fonts/OFL-Inter.txt")),
        (
            "Fira Code",
            include_str!("../../../web/fonts/OFL-FiraCode.txt"),
        ),
    ];

    /// The `unicode-range` of every face: docs-system's `FONT_RANGE` in `src/product-site.ts`.
    pub const FONT_RANGE: &str = "U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2190-21FF,U+2212,U+2215,U+2315,U+25A0-25FF,U+2713,U+2715,U+FEFF,U+FFFD";

    /// The `@font-face` rules with each font as a `data:` URL, every family preceded by its
    /// licence as a comment: the same faces docs-system declares, with nothing loaded from a URL.
    pub fn fonts_css() -> String {
        let mut out = String::new();
        for (family, licence) in LICENCES {
            out.push_str("/*\n");
            out.push_str(&licence.replace("*/", "* /"));
            out.push_str("*/\n");
            for f in FONTS.iter().filter(|f| f.family == family) {
                out.push_str(&format!(
                    "@font-face{{font-family:\"{}\";src:url(\"data:font/woff2;base64,{}\") format(\"woff2\");font-weight:{};font-style:normal;font-display:swap;unicode-range:{FONT_RANGE}}}\n",
                    f.family,
                    base64(f.woff2),
                    f.weight
                ));
            }
        }
        out
    }

    /// Standard base64 with padding (RFC 4648 section 4).
    pub fn base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(char::from(ALPHABET[(n >> (18 - 6 * i)) as usize & 63]));
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn base64_matches_the_rfc_4648_vectors() {
            let vectors = [
                ("", ""),
                ("f", "Zg=="),
                ("fo", "Zm8="),
                ("foo", "Zm9v"),
                ("foob", "Zm9vYg=="),
                ("fooba", "Zm9vYmE="),
                ("foobar", "Zm9vYmFy"),
            ];
            for (plain, encoded) in vectors {
                assert_eq!(base64(plain.as_bytes()), encoded, "{plain:?}");
            }
            assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
        }

        #[test]
        fn every_face_is_a_woff2_data_url_and_every_licence_is_embedded() {
            let css = fonts_css();
            assert_eq!(css.matches("@font-face{").count(), FONTS.len());
            assert_eq!(
                css.matches("src:url(\"data:font/woff2;base64,d09GMg")
                    .count(),
                FONTS.len()
            );
            for f in &FONTS {
                assert!(f.woff2.starts_with(b"wOF2"), "{} is not woff2", f.file);
            }
            for (_, licence) in LICENCES {
                assert!(licence.contains("SIL OPEN FONT LICENSE Version 1.1"));
                assert!(css.contains(licence.lines().next().unwrap_or_default()));
            }
            assert!(
                !css.contains("</"),
                "the stylesheet must not close its <style>"
            );
            assert!(
                !css.contains("url(\"http"),
                "no face is loaded from the network"
            );
        }
    }
}

/// Presentation parameters. Every one is optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Params {
    /// Page title; default the IR's system name.
    pub title: Option<String>,
    /// Web URL of the specification's repository.
    pub repo_url: Option<String>,
    /// The repository link's label; default "Repository".
    pub repo_label: Option<String>,
    /// The commit the page is rendered from; default `git rev-parse HEAD` of the specification.
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    /// Path of the specification inside the repository; default `.`.
    pub spec_root: Option<String>,
    /// A source line's URL: `{repo}`, `{ref}`, `{path}` and `{line}` are replaced.
    /// Default `{repo}/blob/{ref}/{path}#L{line}`.
    pub source_url: Option<String>,
    /// The repository tree's URL: `{repo}`, `{ref}` and `{path}` are replaced.
    /// Default `{repo}/tree/{ref}/{path}`.
    pub tree_url: Option<String>,
    /// The entity whose lifecycle comes first; default the one with the most transitions.
    pub headline_entity: Option<String>,
    /// Section ids in the order wanted; unlisted sections follow in their default order.
    pub section_order: Option<Vec<String>>,
    /// Land on the Demo tab (default) or hide it.
    pub demo: Option<bool>,
    /// Open in presentation mode.
    pub present: Option<bool>,
    /// The authored scenario the Demo plays; default the first.
    pub demo_scenario: Option<String>,
}

/// Everything a page is built from besides the IR.
#[derive(Default)]
pub struct Inputs {
    /// The specification directory: source lines, and the scenario files' `at:` times.
    pub spec_dir: Option<PathBuf>,
    /// Where authored scenario files are read for their `at:` times; default the spec dir.
    pub scenario_dir: Option<PathBuf>,
    /// A synthesized suite.
    pub suite: Option<(String, Value)>,
    /// Authored scenarios.
    pub scenarios: Option<(String, Value)>,
    /// `ess verify diff --format json`.
    pub diff: Option<Value>,
    /// The revision the diff compares against.
    pub diff_ref: Option<String>,
    /// The synthesis log.
    pub synth_log: Option<(String, String)>,
    /// A guard analysis log.
    pub gate_log: Option<(String, String)>,
    /// The compiler version shown in the footer.
    pub compiler: Option<String>,
    /// Where the page will be written, for relative source links.
    pub out: Option<PathBuf>,
    /// Whether the specification's git tree has uncommitted changes.
    pub dirty: bool,
}

/// The presentation data: what the page embeds, and what `ess-ui data` writes.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Presentation {
    /// Always `ess-ui-presentation/1`.
    pub format: String,
    /// The generator and its version.
    pub generator: String,
    /// The system's name.
    pub system: String,
    /// The specification version.
    pub version: String,
    /// The page title.
    pub title: String,
    /// sha256 of the IR bytes.
    pub ir_sha256: String,
    /// The `<title>` of the document.
    pub document_title: String,
    /// Classes of `<body>`: the landing tab, and presentation mode.
    pub body_class: String,
    /// The body's markup: the app bar, the Demo, the Model tab, the inspector and the overlays.
    pub body: String,
    /// The model the scripts read (`#ess-model`): declarations, lifecycles, anchors.
    pub model: Value,
    /// The baked simulation the Demo plays (`#ess-sim`).
    pub sim: Value,
    /// What the page does not draw.
    pub unrendered: Vec<String>,
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// `git rev-parse HEAD` of a directory, and whether its tree has changes.
pub fn git_ref(dir: &Path) -> (Option<String>, bool) {
    let sha = git(dir, &["rev-parse", "HEAD"]);
    let dirty = sha.is_some()
        && git(dir, &["status", "--porcelain", "--", "."]).is_some_and(|s| !s.is_empty());
    (sha, dirty)
}

fn normalize(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    if out.is_empty() {
        ".".into()
    } else {
        out.join("/")
    }
}

fn relative(from_dir: &Path, to: &Path) -> String {
    let abs = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (abs(from_dir), abs(to));
    let ac: Vec<_> = a.components().collect();
    let bc: Vec<_> = b.components().collect();
    let common = ac.iter().zip(&bc).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_owned(), ac.len() - common).collect();
    parts.extend(
        bc[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

fn fill(template: &str, repo: &str, r: &str, path: &str, line: Option<usize>) -> String {
    let t = template
        .replace("{repo}", repo.trim_end_matches('/'))
        .replace("{ref}", r)
        .replace("{path}", path);
    match line {
        Some(l) => t.replace("{line}", &l.to_string()),
        None => t,
    }
}

fn link_maker<'a>(
    p: &'a Params,
    spec_dir: Option<&'a Path>,
    out: Option<&'a Path>,
) -> Box<dyn Fn(&str, usize) -> String + 'a> {
    if let (Some(repo), Some(r)) = (&p.repo_url, &p.git_ref) {
        let root = p.spec_root.clone().unwrap_or_else(|| ".".into());
        let tmpl = p
            .source_url
            .clone()
            .unwrap_or_else(|| "{repo}/blob/{ref}/{path}#L{line}".into());
        return Box::new(move |f, line| {
            fill(
                &tmpl,
                repo,
                r,
                &normalize(&format!("{root}/{f}")),
                Some(line),
            )
        });
    }
    Box::new(move |f, line| {
        let target = spec_dir.map_or_else(|| PathBuf::from(f), |d| d.join(f));
        let base = out.and_then(Path::parent).map_or_else(
            || PathBuf::from("."),
            |p| {
                if p.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    p.to_path_buf()
                }
            },
        );
        format!("{}#L{line}", relative(&base, &target))
    })
}

fn tree_url(p: &Params) -> Option<String> {
    let (repo, r) = (p.repo_url.as_ref()?, p.git_ref.as_ref()?);
    let root = normalize(p.spec_root.as_deref().unwrap_or("."));
    let tmpl = p
        .tree_url
        .clone()
        .unwrap_or_else(|| "{repo}/tree/{ref}/{path}".into());
    let url = fill(&tmpl, repo, r, if root == "." { "" } else { &root }, None);
    Some(url.trim_end_matches('/').to_owned())
}

/// The inspector's facts and related declarations, per anchor.
#[allow(clippy::too_many_lines)]
fn build_decls(m: &Model<'_>) -> Map<String, Value> {
    let ir = m.ir;
    let keys: Vec<((String, String), String)> = {
        let a = m.anchors.borrow();
        let mut v: Vec<_> = a
            .by_key
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        v.sort_by(|x, y| x.1.cmp(&y.1));
        v
    };
    let mut decls = Map::new();
    for ((kind, ident), a) in keys {
        let r = |k: &str, id: &str, label: Option<&str>| {
            json!([k, m.a(k, id), label.unwrap_or_else(|| short(id))])
        };
        let mut t = short(&ident).to_owned();
        let mut s = String::new();
        let mut f: Vec<Value> = Vec::new();
        let mut rr: Vec<Value> = Vec::new();
        let src = m
            .src
            .get(&(kind.clone(), ident.clone()))
            .map(|(file, line)| json!([file, line, (m.link)(file, *line)]));
        match kind.as_str() {
            "system" => {
                t.clone_from(&ident);
                s = summary(ir);
            }
            "component" => {
                let c = m.comps.get(&ident).cloned().unwrap_or(Value::Null);
                t.clone_from(&ident);
                s = summary(&c);
                rr.extend(strs(&c, "owns").iter().map(|x| r("domain", x, Some(x))));
                rr.extend(strs(&c, "accepts").iter().map(|x| r("command", x, None)));
                rr.extend(strs(&c, "publishes").iter().map(|x| r("event", x, None)));
                let mut snd: Vec<String> = strs(&c, "accepts")
                    .iter()
                    .flat_map(|x| m.senders.get(x).cloned().unwrap_or_default())
                    .collect();
                snd.sort();
                snd.dedup();
                rr.extend(snd.iter().map(|x| r("actor", x, None)));
                if let Some(rb) = c.get("reached_by").and_then(Value::as_str) {
                    f.push(json!(["reached_by", rb]));
                }
            }
            "domain" => {
                let d = m.domains.get(&ident).cloned().unwrap_or(Value::Null);
                t.clone_from(&ident);
                s = summary(&d);
                if let Some(o) = m.dom_owner.get(&ident) {
                    rr.push(r("component", o, Some(o)));
                }
                for (key, k2) in [
                    ("entities", "entity"),
                    ("commands", "command"),
                    ("events", "event"),
                    ("views", "view"),
                    ("actors", "actor"),
                    ("errors", "error"),
                ] {
                    rr.extend(strs(&d, key).iter().map(|x| r(k2, x, None)));
                }
            }
            "actor" => {
                let ao = m.actors.get(&ident).cloned().unwrap_or(Value::Null);
                s = display(&ao).unwrap_or_default();
                for at in arr(&ao, "attributes") {
                    f.push(json!([
                        "caller attribute",
                        format!("{}: {}", st(at, "name"), type_label(g(at, "type_ref")))
                    ]));
                }
                rr.extend(strs(&ao, "may").iter().map(|x| r("command", x, None)));
                let mut comps: Vec<String> = strs(&ao, "may")
                    .iter()
                    .flat_map(|x| m.acceptors.get(x).cloned().unwrap_or_default())
                    .collect();
                comps.sort();
                comps.dedup();
                rr.extend(comps.iter().map(|c| r("component", c, Some(c))));
            }
            "entity" => {
                let eo = m.entities.get(&ident).cloned().unwrap_or(Value::Null);
                s = summary(&eo);
                let idf = g(&eo, "identity");
                f.push(json!([
                    "identity",
                    format!("{}: {}", st(idf, "name"), type_label(g(idf, "type_ref")))
                ]));
                for fl in arr(&eo, "fields") {
                    f.push(json!([
                        "field",
                        format!("{}: {}", st(fl, "name"), type_label(g(fl, "type_ref")))
                    ]));
                }
                let dom = st(&eo, "domain");
                rr.push(r("domain", dom, Some(dom)));
                if let Some(o) = m.dom_owner.get(dom) {
                    rr.push(r("component", o, Some(o)));
                }
                rr.extend(
                    strs(g(&eo, "lifecycle"), "states")
                        .iter()
                        .map(|x| r("state", &format!("{ident}#{x}"), Some(x))),
                );
                rr.extend(
                    arr(&eo, "relations")
                        .iter()
                        .map(|x| r("entity", st(x, "target"), None)),
                );
                for (n, o) in m.entities {
                    if n != &ident && arr(o, "relations").iter().any(|x| st(x, "target") == ident) {
                        rr.push(r("entity", n, None));
                    }
                }
                for (cn, effs) in &m.effects_of {
                    if effs.iter().any(|e| e.0 == ident) {
                        rr.push(r("command", cn, None));
                    }
                }
                for (vn, v) in m.views {
                    if st(v, "source") == ident {
                        rr.push(r("view", vn, None));
                    }
                }
            }
            "state" => {
                let (en, sname) = ident.split_once('#').unwrap_or((&ident, ""));
                let lc = m
                    .entities
                    .get(en)
                    .map(|e| g(e, "lifecycle").clone())
                    .unwrap_or(Value::Null);
                t = sname.to_owned();
                let mut flags = Vec::new();
                if sname == st(&lc, "initial") {
                    flags.push("initial");
                }
                if strs(&lc, "terminal").iter().any(|x| x == sname) {
                    flags.push("terminal");
                }
                f.push(json!(["entity", en]));
                if !flags.is_empty() {
                    f.push(json!(["marks", flags.join(", ")]));
                }
                rr.push(r("entity", en, None));
                for tr in arr(&lc, "transitions") {
                    for fr in strs(tr, "from") {
                        let tid = format!("{en}#{}:{fr}->{}", st(tr, "name"), st(tr, "to"));
                        if fr == sname {
                            rr.push(json!([
                                "out",
                                m.a("transition", &tid),
                                format!("{} → {}", st(tr, "name"), st(tr, "to"))
                            ]));
                        }
                        if st(tr, "to") == sname {
                            rr.push(json!([
                                "in",
                                m.a("transition", &tid),
                                format!("{fr} → {}", st(tr, "name"))
                            ]));
                        }
                    }
                }
                for (vn, v) in m.views {
                    let filt = g(v, "filter");
                    if st(v, "source") == en && !filt.is_null() {
                        let ft = predicate_text(filt);
                        let word = ft
                            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                            .any(|w| w == sname);
                        if word {
                            rr.push(r("view", vn, None));
                        }
                    }
                }
            }
            "transition" => {
                let (en, rest) = ident.split_once('#').unwrap_or((&ident, ""));
                let (tname, edge) = rest.split_once(':').unwrap_or((rest, ""));
                let (fr, to) = edge.split_once("->").unwrap_or((edge, ""));
                t = tname.to_owned();
                f.push(json!(["entity", en]));
                f.push(json!(["move", format!("{fr} → {to}")]));
                rr.push(r("state", &format!("{en}#{fr}"), Some(fr)));
                rr.push(r("state", &format!("{en}#{to}"), Some(to)));
                rr.push(r("entity", en, None));
                let causes = m
                    .causes
                    .get(&(en.to_owned(), tname.to_owned()))
                    .cloned()
                    .unwrap_or_default();
                for c in &causes {
                    f.push(json!([
                        "caused by",
                        format!("{} › {} [{}]", short(&c.command), c.outcome, c.condition)
                    ]));
                    rr.push(r("command", &c.command, None));
                    rr.push(r(
                        "outcome",
                        &format!("{}/{}", c.command, c.outcome),
                        Some(&c.outcome),
                    ));
                    rr.extend(c.actors.iter().map(|x| r("actor", x, None)));
                    rr.extend(c.components.iter().map(|x| r("component", x, Some(x))));
                    rr.extend(c.emits.iter().map(|e| r("event", st(e, "event"), None)));
                    for gd in &c.refused_if {
                        f.push(json!(["refused if", st(gd, "condition")]));
                    }
                }
                if causes.is_empty() {
                    f.push(json!(["caused by", "no command outcome in the IR"]));
                }
            }
            "relation" => {
                let (left, target) = ident.rsplit_once("->").unwrap_or((&ident, ""));
                let (src_e, rn) = left.rsplit_once('.').unwrap_or((left, ""));
                let ro = m
                    .entities
                    .get(src_e)
                    .and_then(|e| {
                        arr(e, "relations")
                            .iter()
                            .find(|x| st(x, "name") == rn)
                            .cloned()
                    })
                    .unwrap_or(Value::Null);
                t = rn.to_owned();
                f.push(json!(["kind", st(&ro, "kind")]));
                f.push(json!(["cardinality", text(g(&ro, "cardinality"))]));
                f.push(json!([
                    "via",
                    ro.get("via").and_then(Value::as_str).unwrap_or("—")
                ]));
                rr.push(r("entity", src_e, None));
                rr.push(r("entity", target, None));
            }
            "command" => {
                let c = m.commands.get(&ident).cloned().unwrap_or(Value::Null);
                let nm = g(&c, "naming");
                s = st(nm, "display").to_owned();
                if let Some(w) = nm.get("wire").and_then(Value::as_str) {
                    f.push(json!(["wire", w]));
                }
                let ins: Vec<String> = arr(&c, "input")
                    .iter()
                    .map(|x| format!("{}: {}", st(x, "name"), type_label(g(x, "type_ref"))))
                    .collect();
                f.push(json!([
                    "input",
                    if ins.is_empty() {
                        "—".to_owned()
                    } else {
                        ins.join(", ")
                    }
                ]));
                rr.push(r("domain", st(&c, "domain"), Some(st(&c, "domain"))));
                rr.extend(
                    m.senders
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|x| r("actor", x, None)),
                );
                rr.extend(
                    m.acceptors
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|x| r("component", x, Some(x))),
                );
                for o in arr(&c, "outcomes") {
                    rr.push(r(
                        "outcome",
                        &format!("{ident}/{}", st(o, "name")),
                        Some(st(o, "name")),
                    ));
                    rr.extend(strs(o, "emits").iter().map(|e| r("event", e, None)));
                    if let Some(e) = o.get("error").and_then(Value::as_str) {
                        rr.push(r("error", e, None));
                    }
                    let tr = g(g(o, "subject"), "transition");
                    if tr.is_object() {
                        let en = st(g(o, "subject"), "entity");
                        for fr in strs(tr, "from") {
                            rr.push(json!([
                                "moves",
                                m.a(
                                    "transition",
                                    &format!("{en}#{}:{fr}->{}", st(tr, "name"), st(tr, "to"))
                                ),
                                format!("{}: {fr} → {}", st(tr, "name"), st(tr, "to"))
                            ]));
                        }
                    }
                }
                rr.extend(
                    m.bind_by_command
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|b| r("binding", b, Some(b))),
                );
            }
            "outcome" => {
                let (cn, on) = ident.split_once('/').unwrap_or((&ident, ""));
                let o = m
                    .commands
                    .get(cn)
                    .and_then(|c| {
                        arr(c, "outcomes")
                            .iter()
                            .find(|x| st(x, "name") == on)
                            .cloned()
                    })
                    .unwrap_or(Value::Null);
                t = on.to_owned();
                s = st(&o, "summary").trim().to_owned();
                f.push(json!(["condition", condition_text(g(&o, "condition"))]));
                let subj = g(&o, "subject");
                if subj.is_object() {
                    f.push(json!([
                        "effect",
                        format!("{} {}", st(subj, "effect"), short(st(subj, "entity")))
                    ]));
                    rr.push(r("entity", st(subj, "entity"), None));
                    if st(subj, "effect") == "creates" {
                        let into = subj
                            .get("into")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned)
                            .unwrap_or_else(|| {
                                m.entities
                                    .get(st(subj, "entity"))
                                    .map(|e| st(g(e, "lifecycle"), "initial").to_owned())
                                    .unwrap_or_default()
                            });
                        f.push(json!(["creates into", into]));
                    }
                    let tr = g(subj, "transition");
                    if tr.is_object() {
                        for fr in strs(tr, "from") {
                            rr.push(json!([
                                "moves",
                                m.a(
                                    "transition",
                                    &format!(
                                        "{}#{}:{fr}->{}",
                                        st(subj, "entity"),
                                        st(tr, "name"),
                                        st(tr, "to")
                                    )
                                ),
                                format!("{}: {fr} → {}", st(tr, "name"), st(tr, "to"))
                            ]));
                        }
                    }
                }
                for x in arr(&o, "sets") {
                    f.push(json!(["sets", assign_text(x, true)]));
                }
                rr.insert(0, r("command", cn, None));
                rr.extend(strs(&o, "emits").iter().map(|e| r("event", e, None)));
                if let Some(e) = o.get("error").and_then(Value::as_str) {
                    rr.push(r("error", e, None));
                }
            }
            "event" => {
                let e = m.events.get(&ident).cloned().unwrap_or(Value::Null);
                for x in arr(&e, "fields") {
                    f.push(json!([
                        "field",
                        format!("{}: {}", st(x, "name"), type_label(g(x, "type_ref")))
                    ]));
                }
                rr.push(r("domain", st(&e, "domain"), Some(st(&e, "domain"))));
                rr.extend(
                    m.emitted_by
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|(c, o)| {
                            r(
                                "outcome",
                                &format!("{c}/{o}"),
                                Some(&format!("{} › {o}", short(c))),
                            )
                        }),
                );
                rr.extend(
                    m.publishers
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|x| r("component", x, Some(x))),
                );
                rr.extend(
                    m.bind_by_event
                        .get(&ident)
                        .into_iter()
                        .flatten()
                        .map(|b| r("binding", b, Some(b))),
                );
            }
            "view" => {
                let v = m.views.get(&ident).cloned().unwrap_or(Value::Null);
                s = display(&v).unwrap_or_default();
                if !g(&v, "filter").is_null() {
                    f.push(json!(["filter", predicate_text(g(&v, "filter"))]));
                }
                let ob: Vec<String> = arr(&v, "order_by").iter().map(text).collect();
                if !ob.is_empty() {
                    f.push(json!(["order_by", ob.join(", ")]));
                }
                f.push(json!([
                    "consistency",
                    v.get("consistency").and_then(Value::as_str).unwrap_or("—")
                ]));
                rr.push(r("domain", st(&v, "domain"), Some(st(&v, "domain"))));
                rr.push(r("entity", st(&v, "source"), None));
            }
            "error" => {
                let e = m.errors.get(&ident).cloned().unwrap_or(Value::Null);
                s = st(&e, "summary").trim().to_owned();
                rr.push(r("domain", st(&e, "domain"), Some(st(&e, "domain"))));
                for (cn, c) in m.commands {
                    for o in arr(c, "outcomes") {
                        if st(o, "error") == ident {
                            rr.push(r(
                                "outcome",
                                &format!("{cn}/{}", st(o, "name")),
                                Some(&format!("{} › {}", short(cn), st(o, "name"))),
                            ));
                        }
                    }
                }
            }
            "type" => {
                let b = m
                    .types
                    .get(&ident)
                    .map(|x| g(x, "body").clone())
                    .unwrap_or(Value::Null);
                f.push(json!(["kind", st(&b, "kind")]));
                let vs: Vec<String> = arr(&b, "variants")
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map_or_else(|| st(v, "name").to_owned(), ToOwned::to_owned)
                    })
                    .collect();
                if !vs.is_empty() {
                    f.push(json!(["variants", vs.join(", ")]));
                }
                if b.get("of").is_some() {
                    f.push(json!(["of", type_label(g(&b, "of"))]));
                }
            }
            "binding" => {
                let b = m.bindings.get(&ident).cloned().unwrap_or(Value::Null);
                t.clone_from(&ident);
                s = summary(&b);
                rr.push(r("event", st(&b, "event"), None));
                rr.push(r("command", st(&b, "command"), None));
                if let Some(e) = b.get("escalation").and_then(Value::as_str) {
                    rr.push(r("event", e, None));
                }
            }
            "workload" => {
                let w = m.workloads.get(&ident).cloned().unwrap_or(Value::Null);
                t.clone_from(&ident);
                rr.push(r(
                    "component",
                    st(&w, "component"),
                    Some(st(&w, "component")),
                ));
            }
            _ => {}
        }
        let mut seen = std::collections::BTreeSet::new();
        let rr: Vec<Value> = rr
            .into_iter()
            .filter(|x| {
                let id = x[1].as_str().unwrap_or("").to_owned();
                id != a && seen.insert(id)
            })
            .collect();
        decls.insert(
            a.clone(),
            json!({"k": kind, "i": ident, "t": t, "s": s, "f": f, "r": rr, "src": src}),
        );
    }
    decls
}

fn page_sets(m: &Model<'_>) -> Value {
    let mut states = Vec::new();
    let mut trans = Vec::new();
    for (n, e) in m.entities {
        let lc = g(e, "lifecycle");
        states.extend(strs(lc, "states").iter().map(|s| format!("{n}#{s}")));
        for t in arr(lc, "transitions") {
            trans.extend(
                strs(t, "from")
                    .iter()
                    .map(|f| format!("{n}#{}:{f}->{}", st(t, "name"), st(t, "to"))),
            );
        }
    }
    states.sort();
    trans.sort();
    json!({"components": m.comps.keys().collect::<Vec<_>>(), "domains": m.domains.keys().collect::<Vec<_>>(), "entities": m.entities.keys().collect::<Vec<_>>(),
        "commands": m.commands.keys().collect::<Vec<_>>(), "states": states, "transitions": trans})
}

const REPO_ICON: &str = r#"<svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="5" r="2.2"/><circle cx="6" cy="19" r="2.2"/><circle cx="18" cy="8" r="2.2"/><path d="M6 7.2v9.6M18 10.2c0 4-6 3.2-11 6.6"/></svg>"#;

/// Builds the presentation from the IR text, the baked simulation and the parameters.
#[allow(clippy::too_many_lines)]
pub fn build(
    raw_ir: &str,
    baked: &Baked,
    params: &Params,
    inputs: &Inputs,
) -> anyhow::Result<Presentation> {
    let ir: Value = serde_json::from_str(raw_ir)?;
    let digest = hex(&Sha256::digest(raw_ir.as_bytes()));
    let mut unr = Unrendered::default();
    let mut m = Model::build(&ir, &mut unr);
    let (marks, diff_items) = diff_index(inputs.diff.as_ref());
    m.chg = marks;
    let (loc, missing, fmt_src) = match &inputs.spec_dir {
        Some(dir) => {
            let idx = SourceIndex::new(dir);
            for f in &idx.missing_files {
                unr.add(format!("source file `{f}` listed by the specification does not exist; its declarations have no links"));
            }
            let (loc, missing) = idx.locate_all(&ir);
            (loc, missing, idx.format.clone())
        }
        None => (BTreeMap::new(), Vec::new(), None),
    };
    if inputs.spec_dir.is_none() {
        unr.add("no specification directory was given (`--spec-dir`); declarations have no source links".to_owned());
    }
    m.src = loc;
    m.link = link_maker(params, inputs.spec_dir.as_deref(), inputs.out.as_deref());
    for (kind, ident) in &missing {
        unr.add(format!(
            "no source line located for {kind} `{ident}`; it has no source link"
        ));
    }
    let sysname = st(&ir, "system").to_owned();
    let version = text(g(&ir, "version"));
    let (fmt, fmt_from) = match ir.get("format").and_then(Value::as_str) {
        Some(f) => (Some(f.to_owned()), Some("IR".to_owned())),
        None => match &fmt_src {
            Some((f, file, line)) => (Some(f.clone()), Some(format!("{file}:{}", line + 1))),
            None => (None, None),
        },
    };
    let hero_tag = tag(&m, "system", &sysname, true, &[]);
    let (arch, colour) = render_architecture(&m);
    let mut lc_ents: Vec<String> = m
        .entities
        .iter()
        .filter(|(_, e)| strs(g(e, "lifecycle"), "states").len() > 1)
        .map(|(n, _)| n.clone())
        .collect();
    lc_ents.sort_by_key(|n| {
        std::cmp::Reverse(arr(g(&m.entities[n], "lifecycle"), "transitions").len())
    });
    if let Some(head) = &params.headline_entity {
        if let Some(i) = lc_ents.iter().position(|x| x == head) {
            let h = lc_ents.remove(i);
            lc_ents.insert(0, h);
        } else {
            unr.add(format!("parameter headline_entity `{head}` is not an entity with more than one state; default headline used"));
        }
    }
    let mut lc_html = String::new();
    let mut lc_js = Vec::new();
    for (i, n) in lc_ents.iter().enumerate() {
        let sid = format!("lc-{}", slug(n));
        let (svg, js) = render_lifecycle(&m, n, &sid, &mut unr, i == 0);
        let lc = g(&m.entities[n], "lifecycle");
        let d = st(&m.entities[n], "domain");
        let ea = m.a("entity", n);
        let paths = arr(&js, "paths").to_vec();
        lc_html.push_str(&format!(
            r##"<div class="lc-block" id="{sid}-block" data-title="{}"><h3 class="dom-h" style="--dc:{}"><i class="dsw"></i><a class="ref" href="#{}" data-ref="{}">{}</a> · {} states · {} transition edges · {} shortest path(s) to terminal {}</h3><div class="lc-wrap"><div class="panel">{svg}</div>{}</div></div>"##,
            esc(n), m.dom_colour.get(d).map_or("#888", String::as_str), esc(&ea), esc(&ea), esc(n), strs(lc, "states").len(), arr(&js, "edges").len(), paths.len(), src_link(&m, "entity", n), live_panel(&sid, n, &paths)
        ));
        lc_js.push(js);
    }
    for n in arr(&baked.sim, "notes") {
        unr.add(text(n));
    }
    let demo_html = render_demo(&m, &baked.sim, &colour);
    let er = render_er(&m);
    let (branches, n_branch) = render_branches(&m);
    let commands = render_commands(&m, &colour);
    let events = render_events(&m);
    let views = render_views(&m);
    let inter = render_interactions(&m);
    let errtypes = render_errors_types(&m);
    let srcidx = render_source_index(&m, missing.len());
    let legend = legend_html(&m);
    let n_inter = m.bindings.len() + m.conversions.len() + m.workloads.len();
    let n_states: usize = m
        .entities
        .values()
        .map(|e| strs(g(e, "lifecycle"), "states").len())
        .sum();
    let n_trans: usize = m
        .entities
        .values()
        .map(|e| arr(g(e, "lifecycle"), "transitions").len())
        .sum();
    let counts: Vec<(&str, usize)> = vec![
        ("components", m.comps.len()),
        ("domains", m.domains.len()),
        ("actors", m.actors.len()),
        ("entities", m.entities.len()),
        ("states", n_states),
        ("transitions", n_trans),
        ("commands", m.commands.len()),
        ("events", m.events.len()),
        ("views", m.views.len()),
    ];
    let gate = parse_gate(
        inputs
            .synth_log
            .as_ref()
            .map(|(p, t)| (p.as_str(), t.as_str())),
        inputs
            .gate_log
            .as_ref()
            .map(|(p, t)| (p.as_str(), t.as_str())),
        inputs.suite.as_ref().map(|(p, _)| p.clone()),
        inputs.scenarios.as_ref().map(|(p, _)| p.clone()),
    );
    let head_lc = lc_ents.first().map_or_else(|| "—".to_owned(), |x| esc(x));
    let overview = render_overview(&counts, &gate, &baked.sim);
    let simulation = render_simulation(&m, &baked.sim);
    let changes = render_changes(
        &m,
        inputs.diff.as_ref(),
        inputs.diff_ref.as_deref(),
        &diff_items,
    );
    for s in params.section_order.iter().flatten() {
        if !DEFAULT_ORDER.contains(&s.as_str()) {
            unr.add(format!(
                "parameter section_order names unknown section `{s}`; known: {}",
                DEFAULT_ORDER.join(", ")
            ));
        }
    }
    let unr_items = unr.sorted();
    let unr_html = if unr_items.is_empty() {
        r#"<p class="dim">The generator met no IR key or construct it does not render.</p>"#
            .to_owned()
    } else {
        format!(
            r#"<ul class="unr">{}</ul>"#,
            unr_items
                .iter()
                .map(|x| format!("<li>{}</li>", esc(x)))
                .collect::<String>()
        )
    };
    let n_traces = arr(&baked.sim, "traces").len();
    let mut sections: BTreeMap<&str, (&str, usize, String, String)> = BTreeMap::new();
    sections.insert("overview", ("Overview", counts.len(), "Declaration counts from the IR and the gate results the build passed in. Each number names where it came from.".into(), overview));
    sections.insert(
        "simulation",
        ("Simulation", n_traces, String::new(), simulation),
    );
    sections.insert(
        "changes",
        ("Changes", diff_items.len(), String::new(), changes),
    );
    sections.insert("arch", ("Architecture", m.comps.len(), "Actors on top, then each component in the order its entities relate to the next, with the domains it owns (in their domain colour) and their entities. Coloured links: the components that accept the commands an actor may send. During a lifecycle walk the sending actor and the accepting component light up.".into(), format!("{}{arch}", domain_legend(&m))));
    sections.insert("lifecycle", ("Lifecycles", lc_ents.len(), format!("Every entity whose lifecycle has more than one state; the first is <code>{head_lc}</code>. Each edge carries its transition name, the command › outcome that moves it, its guard and the actors that may send the command. Pick a shortest path to a terminal state, or let the token tour every transition."), if lc_html.is_empty() { r#"<p class="dim">No entity has more than one state.</p>"#.into() } else { lc_html }));
    sections.insert("branches", ("Guarded branches", n_branch, "Commands whose outcomes branch on a guard. Enum guards are split into the variants each outcome takes; <code>otherwise</code> takes the variants no guard claims. Guards that are not enum splits are shown as their text.".into(), branches));
    sections.insert("entities", ("Entities", m.entities.len(), "Rows are components, in architecture order. Each box: identity, fields, invariants, relations, <code>updates</code> outcomes (Δ) and lifecycle states.".into(), format!(r#"<div class="panel">{er}</div>"#)));
    sections.insert("commands", ("Commands", m.commands.len(), "Per command: who may send it, who accepts it, and every outcome with its condition, effect, emitted events (payload fields in brackets), refusal or error.".into(), commands));
    sections.insert("events", ("Events", m.events.len(), String::new(), events));
    sections.insert("views", ("Views", m.views.len(), String::new(), views));
    sections.insert(
        "interactions",
        (
            "Interactions",
            n_inter,
            "Bindings from events to commands, type conversions and workloads.".into(),
            inter,
        ),
    );
    sections.insert(
        "errors",
        (
            "Errors and types",
            m.errors.len() + m.types.len(),
            String::new(),
            errtypes,
        ),
    );
    sections.insert(
        "sources",
        (
            "Sources",
            m.src.len(),
            "Where each declaration is written.".into(),
            srcidx,
        ),
    );
    sections.insert("legend", ("Legend", m.domains.len(), String::new(), legend));
    sections.insert(
        "coverage",
        (
            "Not rendered",
            unr_items.len(),
            "What the generator does not draw or could not locate. Computed at generation time."
                .into(),
            unr_html,
        ),
    );
    let mut order: Vec<&str> = params
        .section_order
        .iter()
        .flatten()
        .filter_map(|s| DEFAULT_ORDER.iter().find(|d| **d == s.as_str()).copied())
        .collect();
    for d in DEFAULT_ORDER {
        if !order.contains(&d) {
            order.push(d);
        }
    }
    let decls = build_decls(&m);
    let stats: String = counts
        .iter()
        .map(|(k, v)| format!(r#"<div class="stat"><b>{v}</b>{k}</div>"#))
        .collect();
    let anchors_actor: Map<String, Value> = m
        .actors
        .keys()
        .map(|a| (a.clone(), json!(m.a("actor", a))))
        .collect();
    let anchors_comp: Map<String, Value> = m
        .comps
        .keys()
        .map(|c| (c.clone(), json!(m.a("component", c))))
        .collect();
    let mut model_json = json!({
        "system": sysname, "version": version, "format": fmt, "ir_sha256": digest, "compiler": inputs.compiler,
        "params": params, "dirty": inputs.dirty, "lifecycles": lc_js, "decls": decls,
        "anchor": {"actor": anchors_actor, "component": anchors_comp},
        "unlocated": missing.iter().map(|(k, i)| json!([k, i])).collect::<Vec<_>>(), "sets": page_sets(&m),
    });
    model_json = serde_json::from_str(&serde_json::to_string(&model_json)?)?;
    let nav: String = order
        .iter()
        .map(|sid| {
            format!(
                r##"<a class="sec" href="#{sid}">{}<i>{}</i></a>"##,
                esc(sections[sid].0),
                sections[sid].1
            )
        })
        .collect();
    let body_main: String = order
        .iter()
        .enumerate()
        .map(|(i, sid)| {
            let (title, _, lede, html) = &sections[sid];
            format!(
                r#"<section id="{sid}" data-title="{}"><h2><span class="idx">{:02}</span>{}</h2>{}{html}</section>"#,
                esc(title), i + 1, esc(title), if lede.is_empty() { String::new() } else { format!("<p class=lede>{lede}</p>") }
            )
        })
        .collect();
    let tree = tree_url(params);
    let gref = params.git_ref.clone();
    let ref_html = match &gref {
        Some(r) => format!(
            r#"<span class="ref">{}</span>{}"#,
            esc(&r.chars().take(8).collect::<String>()),
            if inputs.dirty {
                r#" <span class="dirty">dirty</span>"#
            } else {
                ""
            }
        ),
        None => r#"<span class="dim">no ref</span>"#.into(),
    };
    let label = params
        .repo_label
        .clone()
        .unwrap_or_else(|| "Repository".into());
    let corner = format!(
        r#"<div class="corner"><div class="who"><span>ref {ref_html}</span></div>{}</div>"#,
        tree.as_ref().map(|t| format!(
            r#"<a class="repo" href="{}" title="Open the specification's repository at {}" aria-label="Open the repository at the rendered ref">{REPO_ICON}<span>{}</span></a>"#,
            esc(t), esc(gref.as_deref().unwrap_or("")), esc(&label)
        )).unwrap_or_default()
    );
    let title = params.title.clone().unwrap_or_else(|| sysname.clone());
    let foot = format!(
        r#"<span>system <b>{}</b></span><span>version <b>{}</b></span><span>format <b>{}</b>{}</span><span>compiler <b>{}</b></span><span>ref <b>{}{}</b></span><span>IR sha256 <span class="hash" id="ir-digest">{digest}</span></span>"#,
        esc(&sysname),
        esc(&version),
        fmt.as_deref().map_or_else(|| "not found".to_owned(), esc),
        fmt_from
            .map(|s| format!(" ({})", esc(&s)))
            .unwrap_or_default(),
        esc(inputs.compiler.as_deref().unwrap_or("unknown")),
        esc(gref.as_deref().unwrap_or("none")),
        if inputs.dirty { " (dirty)" } else { "" }
    );
    let demo_on = params.demo.unwrap_or(true);
    let present = params.present.unwrap_or(false);
    let (demo_cls, model_cls) = if demo_on {
        (r#" class="on""#, "")
    } else {
        ("", r#" class="on""#)
    };
    let body_class = format!(
        "{}{}",
        if demo_on {
            "tab-demo"
        } else {
            "tab-model demo-off"
        },
        if present { " present-auto" } else { "" }
    );
    let mark: String = title
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default();
    let appbar = format!(
        r#"<div id="appbar" class="appbar"><span class="ab-brand"><span class="ab-mark" aria-hidden="true">{}</span><span class="ab-title">{}</span><span class="ab-sub">/ ESS</span></span><span class="ab-tabs" role="tablist" aria-label="View"><button type="button" role="tab" data-tab="model"{model_cls}>Model</button><button type="button" role="tab" data-tab="demo"{demo_cls}>Demo</button></span><span class="ab-tools"><button type="button" id="palette-btn" title="Command palette (Ctrl+K)">⌘K</button><button type="button" id="theme2" title="Theme" aria-label="Toggle theme"><span class="theme-glyph" aria-hidden="true"></span></button></span>{corner}</div>"#,
        esc(&mark),
        esc(&title)
    );
    let sys_summary = summary(&ir);
    let body = format!(
        r#"{defs}
{appbar}
{demo_html}
<header class="hero">{corner}<div class="kicker">ESS specification · compiled IR</div>
<h1 {hero_tag}>{title_e}</h1>
<div class="lede">{summ}Rendered from IR <code>{d16}…</code>. Every name on this page is read from the IR at generation time. Keys: <code>/</code> search · <code>p</code> present · <code>Esc</code> close.</div>
<div class="stats">{stats}</div></header>
<nav aria-label="Sections">{nav}<span class="tools"><input id="q" type="search" placeholder="Search declarations  /" aria-label="Search declarations">
<span class="qcount" id="qcount"></span><button id="theme" type="button">Light</button><button id="present" type="button" title="Presentation mode (p)">Present</button></span></nav>
<main>{body_main}</main><footer>{foot}</footer>
<aside id="inspector" aria-label="Inspector" aria-hidden="true"></aside>
<div id="slidebar" role="toolbar" aria-label="Slides"><button class="prev" type="button" aria-label="Previous">←</button><span class="n"></span>
<span class="t"></span><button class="next" type="button" aria-label="Next">→</button><span class="hint"></span><button class="end" type="button">Esc</button></div>
<div id="palette" class="palette" hidden role="dialog" aria-label="Command palette"><input id="palette-q" type="search" placeholder="Jump to a declaration or run an action" aria-label="Command palette search"><ol id="palette-list"></ol></div>"#,
        defs = assets::DEFS.trim(),
        title_e = esc(&title),
        summ = if sys_summary.is_empty() {
            String::new()
        } else {
            format!("{} ", esc(&sys_summary))
        },
        d16 = &digest[..16],
    );
    let mut sim = baked.sim.clone();
    if let Some(o) = sim.as_object_mut() {
        o.insert("entity".into(), Value::Object(m.entities.iter().map(|(n, e)| (n.clone(), json!({"initial": st(g(e, "lifecycle"), "initial"), "domain": st(e, "domain"), "a": m.a("entity", n)}))).collect()));
        o.insert("views".into(), Value::Object(m.views.iter().map(|(v, vo)| (v.clone(), json!({"fields": arr(vo, "fields").iter().map(|f| st(f, "name")).collect::<Vec<_>>(), "domain": st(vo, "domain"), "a": m.a("view", v)}))).collect()));
        o.insert(
            "actors".into(),
            Value::Object(
                m.actors
                    .keys()
                    .map(|a| {
                        (
                            a.clone(),
                            json!({"colour": colour.get(a), "a": m.a("actor", a)}),
                        )
                    })
                    .collect(),
            ),
        );
        o.insert(
            "components".into(),
            Value::Object(
                m.comps
                    .keys()
                    .map(|c| (c.clone(), json!(m.a("component", c))))
                    .collect(),
            ),
        );
        o.insert(
            "domains".into(),
            Value::Object(
                m.domains
                    .keys()
                    .map(|d| {
                        (
                            d.clone(),
                            json!({"colour": m.dom_colour.get(d), "a": m.a("domain", d)}),
                        )
                    })
                    .collect(),
            ),
        );
        o.insert(
            "events".into(),
            Value::Object(
                m.events
                    .iter()
                    .map(|(e, eo)| (e.clone(), json!(st(eo, "domain"))))
                    .collect(),
            ),
        );
    }
    Ok(Presentation {
        format: FORMAT.into(),
        generator: format!("ess-ui {}", env!("CARGO_PKG_VERSION")),
        system: sysname.clone(),
        version,
        title,
        ir_sha256: digest.clone(),
        document_title: format!("{sysname} · ESS IR {}", &digest[..12]),
        body_class,
        body,
        model: model_json,
        sim,
        unrendered: unr_items,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn script_json(v: &Value) -> String {
    serde_json::to_string(v)
        .unwrap_or_default()
        .replace("</", "<\\/")
}

/// The self-contained page: the presentation, the stylesheet and the scripts in one document.
pub fn html(p: &Presentation) -> String {
    format!(
        r#"<!doctype html>
<html lang="en" data-theme="light"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="ess-ir-sha256" content="{digest}"><meta name="generator" content="{gen}">
<title>{title}</title><script>{head}</script><style>{fonts}{css}</style></head>
<body class="{class}">{body}
<script type="application/json" id="ess-model">{model}</script>
<script type="application/json" id="ess-sim">{sim}</script>
<script>{js}</script></body></html>
"#,
        digest = p.ir_sha256,
        gen = esc(&p.generator),
        title = esc(&p.document_title),
        head = assets::HEAD_JS.trim(),
        fonts = assets::fonts_css(),
        css = assets::css(),
        class = esc(&p.body_class),
        body = p.body,
        model = script_json(&p.model),
        sim = script_json(&p.sim),
        js = assets::js()
    )
}
