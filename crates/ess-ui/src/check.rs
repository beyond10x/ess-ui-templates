//! Holds a generated page to the IR it was drawn from.
//!
//! Reads the page as HTML, without the generator's rendering code, so a rendering bug cannot hide
//! itself:
//!
//! 1. **Sets.** For each kind (component, domain, actor, entity, state, transition, relation,
//!    command, outcome, event, view), the elements tagged on the page are the IR's set.
//! 2. **Digest.** The IR sha256 embedded in the page is the sha256 of the IR.
//! 3. **Source lines.** Every source link names a line of the specification that declares the name,
//!    and its href ends in `<file>#L<line>`.
//! 4. **Link coverage.** Every tagged declaration has a source link or is listed as unlocated.
//! 5. **Not rendered.** The page lists nothing it does not draw.
//! 6. **Examples.** Every `example:` of a command is shown on its card.
//! 7. **Replay.** Every scenario replays with every expectation met, and ESS's interpreter and the
//!    page's never read a step differently.
//! 8. **Demo script.** The script the page plays is the script a fresh bake produces.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// One check's result.
#[derive(Debug, Clone)]
pub struct Line {
    /// Passed.
    pub ok: bool,
    /// What it says.
    pub text: String,
}

/// Every check's result.
#[derive(Debug, Default)]
pub struct Report {
    /// The lines, in order.
    pub lines: Vec<Line>,
}

impl Report {
    fn push(&mut self, ok: bool, text: impl Into<String>) {
        self.lines.push(Line {
            ok,
            text: text.into(),
        });
    }

    /// Whether every check passed.
    pub fn passed(&self) -> bool {
        self.lines.iter().all(|l| l.ok)
    }

    /// The report as text, one line per check, and a result line.
    pub fn render(&self) -> String {
        let mut s: String = self
            .lines
            .iter()
            .map(|l| format!("{} {}\n", if l.ok { "ok  " } else { "FAIL" }, l.text))
            .collect();
        let fails = self.lines.iter().filter(|l| !l.ok).count();
        s.push_str(&if fails == 0 {
            "RESULT: PASS\n".to_owned()
        } else {
            format!("RESULT: FAIL ({fails} failing check(s))\n")
        });
        s
    }
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// One start tag: its name and attributes.
pub struct Tag {
    /// Lower-case name.
    pub name: String,
    /// Attributes, unescaped.
    pub attrs: BTreeMap<String, String>,
    /// Byte offset of the tag in the page.
    pub at: usize,
}

/// Every start tag of a page, and the text of `<script id=…>` elements by id.
pub fn scan(html: &str) -> (Vec<Tag>, BTreeMap<String, String>) {
    let b = html.as_bytes();
    let mut tags = Vec::new();
    let mut scripts = BTreeMap::new();
    let mut i = 0;
    while let Some(off) = html[i..].find('<') {
        let start = i + off;
        let rest = &html[start + 1..];
        if rest.starts_with('/') || rest.starts_with('!') {
            i = start + 1;
            continue;
        }
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        let mut j = start + 1 + name_end;
        let mut attrs = BTreeMap::new();
        loop {
            while j < b.len() && (b[j] as char).is_whitespace() {
                j += 1;
            }
            if j >= b.len() || b[j] == b'>' || b[j] == b'/' {
                break;
            }
            let k0 = j;
            while j < b.len() && !(b[j] as char).is_whitespace() && b[j] != b'=' && b[j] != b'>' {
                j += 1;
            }
            let key = html[k0..j].to_ascii_lowercase();
            let mut val = String::new();
            if j < b.len() && b[j] == b'=' {
                j += 1;
                if j < b.len() && (b[j] == b'"' || b[j] == b'\'') {
                    let q = b[j];
                    let v0 = j + 1;
                    j = v0;
                    while j < b.len() && b[j] != q {
                        j += 1;
                    }
                    val = unescape(&html[v0..j]);
                    j += 1;
                } else {
                    let v0 = j;
                    while j < b.len() && !(b[j] as char).is_whitespace() && b[j] != b'>' {
                        j += 1;
                    }
                    val = unescape(&html[v0..j]);
                }
            }
            attrs.insert(key, val);
        }
        let end = html[j..].find('>').map_or(html.len(), |e| j + e + 1);
        if name == "script" {
            let close = html[end..]
                .find("</script>")
                .map_or(html.len(), |c| end + c);
            if let Some(id) = attrs.get("id") {
                scripts.insert(id.clone(), html[end..close].to_owned());
            }
            i = close;
        } else {
            i = end;
        }
        tags.push(Tag {
            name,
            attrs,
            at: start,
        });
    }
    (tags, scripts)
}

fn expected(ir: &Value) -> BTreeMap<&'static str, BTreeSet<String>> {
    let keys = |k: &str| -> BTreeSet<String> {
        ir.get(k)
            .and_then(Value::as_object)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    };
    let mut exp = BTreeMap::from([
        ("component", keys("components")),
        ("domain", keys("domains")),
        ("actor", keys("actors")),
        ("entity", keys("entities")),
        ("command", keys("commands")),
        ("event", keys("events")),
        ("view", keys("views")),
        ("state", BTreeSet::new()),
        ("transition", BTreeSet::new()),
        ("relation", BTreeSet::new()),
        ("outcome", BTreeSet::new()),
    ]);
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
    let a = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for (n, e) in ir
        .get("entities")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let lc = e.get("lifecycle").cloned().unwrap_or(Value::Null);
        for st in a(&lc, "states") {
            exp.get_mut("state")
                .unwrap()
                .insert(format!("{n}#{}", st.as_str().unwrap_or("")));
        }
        for t in a(&lc, "transitions") {
            for f in a(&t, "from") {
                exp.get_mut("transition").unwrap().insert(format!(
                    "{n}#{}:{}->{}",
                    s(&t, "name"),
                    f.as_str().unwrap_or(""),
                    s(&t, "to")
                ));
            }
        }
        for r in a(e, "relations") {
            exp.get_mut("relation").unwrap().insert(format!(
                "{n}.{}->{}",
                s(&r, "name"),
                s(&r, "target")
            ));
        }
    }
    for (cn, c) in ir
        .get("commands")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        for o in a(c, "outcomes") {
            exp.get_mut("outcome")
                .unwrap()
                .insert(format!("{cn}/{}", s(&o, "name")));
        }
    }
    exp
}

fn decl_name(kind: &str, ident: &str) -> String {
    match kind {
        "state" => ident.split_once('#').map_or(ident, |x| x.1).to_owned(),
        "transition" => ident
            .split_once('#')
            .map_or(ident, |x| x.1)
            .split(':')
            .next()
            .unwrap_or("")
            .to_owned(),
        "outcome" => ident.split_once('/').map_or(ident, |x| x.1).to_owned(),
        "relation" => ident
            .rsplit_once("->")
            .map_or(ident, |x| x.0)
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_owned(),
        "type" if ident.ends_with(".State") => ident.trim_end_matches(".State").to_owned(),
        _ => ident.to_owned(),
    }
}

fn line_declares(
    spec_dir: &Path,
    file: &str,
    line: usize,
    name: &str,
    cache: &mut BTreeMap<String, Option<Vec<String>>>,
) -> Option<String> {
    let lines = cache.entry(file.to_owned()).or_insert_with(|| {
        std::fs::read_to_string(spec_dir.join(file))
            .ok()
            .map(|t| t.split('\n').map(ToOwned::to_owned).collect())
    });
    let Some(lines) = lines else {
        return Some(format!("file {file} does not exist"));
    };
    if line == 0 || line > lines.len() {
        return Some(format!("{file} has {} lines, not {line}", lines.len()));
    }
    let text = &lines[line - 1];
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    let found = text.match_indices(name).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = &text[i + name.len()..];
        let a0 = after.chars().next();
        !before.is_some_and(|c| word(c) || c == '.')
            && !a0.is_some_and(word)
            && !(a0 == Some('.')
                && after
                    .chars()
                    .nth(1)
                    .is_some_and(|c| c.is_alphanumeric() || c == '_'))
    });
    (!found).then(|| {
        format!(
            "{file}:{line} is `{}`, which does not contain `{name}`",
            text.trim()
        )
    })
}

const LINK_KINDS: [&str; 13] = [
    "component",
    "domain",
    "actor",
    "entity",
    "state",
    "transition",
    "relation",
    "command",
    "outcome",
    "event",
    "view",
    "error",
    "type",
];

/// Checks `html` against the IR text and the specification directory.
#[allow(clippy::too_many_lines)]
pub fn check(ir_text: &str, html: &str, spec_dir: Option<&Path>) -> Report {
    let mut rep = Report::default();
    let ir: Value = serde_json::from_str(ir_text).unwrap_or(Value::Null);
    let digest: String = Sha256::digest(ir_text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let (tags, scripts) = scan(html);
    let model: Value = scripts
        .get("ess-model")
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or(Value::Null);
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for t in &tags {
        if let (Some(k), Some(i)) = (t.attrs.get("data-ess-kind"), t.attrs.get("data-ess-id")) {
            found.entry(k.clone()).or_default().insert(i.clone());
        }
    }
    for (kind, want) in expected(&ir) {
        let got = found.get(kind).cloned().unwrap_or_default();
        let missing: Vec<&String> = want.difference(&got).collect();
        let extra: Vec<&String> = got.difference(&want).collect();
        if missing.is_empty() && extra.is_empty() {
            rep.push(true, format!("{kind}: {} == {}", want.len(), got.len()));
        } else {
            rep.push(false, format!("{kind}: IR {}, page {}; missing from page: {missing:?}; on page but not in IR: {extra:?}", want.len(), got.len()));
        }
    }
    let meta = tags
        .iter()
        .find(|t| {
            t.name == "meta" && t.attrs.get("name").map(String::as_str) == Some("ess-ir-sha256")
        })
        .and_then(|t| t.attrs.get("content").cloned());
    let embedded = model
        .get("ir_sha256")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    rep.push(
        meta.as_deref() == Some(&digest) && embedded.as_deref() == Some(&digest),
        format!("ir digest: IR {digest}, page meta {meta:?}, page model {embedded:?}"),
    );

    let mut linked: BTreeSet<(String, String)> = BTreeSet::new();
    let mut entries: Vec<(&str, String, String, String, String, String)> = Vec::new();
    for t in tags.iter().filter(|t| {
        t.name == "a"
            && t.attrs
                .get("class")
                .is_some_and(|c| c.split_whitespace().any(|x| x == "src"))
    }) {
        let g = |k: &str| t.attrs.get(k).cloned().unwrap_or_default();
        entries.push((
            "page",
            g("data-src-kind"),
            g("data-src-id"),
            g("data-src-file"),
            g("data-src-line"),
            g("href"),
        ));
    }
    let n_page = entries.len();
    for d in model
        .get("decls")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .map(|(_, d)| d)
    {
        if let Some(src) = d.get("src").and_then(Value::as_array) {
            let s = |i: usize| {
                src.get(i)
                    .map(|v| v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned))
                    .unwrap_or_default()
            };
            entries.push((
                "inspector",
                d["k"].as_str().unwrap_or("").to_owned(),
                d["i"].as_str().unwrap_or("").to_owned(),
                s(0),
                s(1),
                s(2),
            ));
        }
    }
    match spec_dir {
        Some(dir) => {
            let mut cache = BTreeMap::new();
            let mut bad = Vec::new();
            for (where_, kind, ident, file, line, href) in &entries {
                let Ok(line) = line.parse::<usize>() else {
                    bad.push(format!(
                        "{where_} {kind} {ident}: line `{line}` is not a number"
                    ));
                    continue;
                };
                if let Some(err) =
                    line_declares(dir, file, line, &decl_name(kind, ident), &mut cache)
                {
                    bad.push(format!("{where_} {kind} {ident}: {err}"));
                }
                if !href.ends_with(&format!("{file}#L{line}")) {
                    bad.push(format!(
                        "{where_} {kind} {ident}: href `{href}` does not end in `{file}#L{line}`"
                    ));
                }
                if *where_ == "page" {
                    linked.insert((kind.clone(), ident.clone()));
                }
            }
            rep.push(
                bad.is_empty(),
                if bad.is_empty() {
                    format!(
                        "source links: {} checked ({n_page} on the page, {} in the inspector data)",
                        entries.len(),
                        entries.len() - n_page
                    )
                } else {
                    format!(
                        "source links: {} of {} wrong: {:?}",
                        bad.len(),
                        entries.len(),
                        &bad[..bad.len().min(10)]
                    )
                },
            );
            let unlocated: BTreeSet<(String, String)> = model
                .get("unlocated")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|x| {
                    Some((
                        x.get(0)?.as_str()?.to_owned(),
                        x.get(1)?.as_str()?.to_owned(),
                    ))
                })
                .collect();
            let cov_start = tags
                .iter()
                .find(|t| {
                    t.name == "section" && t.attrs.get("id").map(String::as_str) == Some("coverage")
                })
                .map_or(html.len(), |t| t.at);
            let cov_text = unescape(
                &html[cov_start
                    ..html[cov_start..]
                        .find("</section>")
                        .map_or(html.len(), |e| cov_start + e)],
            );
            let mut uncovered = Vec::new();
            for kind in LINK_KINDS {
                for ident in found.get(kind).into_iter().flatten() {
                    let key = (kind.to_owned(), ident.clone());
                    if linked.contains(&key)
                        || (unlocated.contains(&key)
                            && cov_text.contains(&format!("{kind} `{ident}`")))
                    {
                        continue;
                    }
                    uncovered.push(format!("{kind} {ident}"));
                }
            }
            rep.push(uncovered.is_empty(), if uncovered.is_empty() { format!("link coverage: every tagged declaration is linked or listed as unlocated ({} unlocated)", unlocated.len()) } else { format!("link coverage: {} tagged declaration(s) have no source link and are not listed as unlocated: {:?}", uncovered.len(), &uncovered[..uncovered.len().min(10)]) });
        }
        None => rep.push(
            true,
            "source links: no specification directory given; not checked",
        ),
    }
    let unr = html.find(r#"<ul class="unr">"#).map(|s| {
        let e = html[s..].find("</ul>").map_or(html.len(), |e| s + e);
        html[s..e].matches("<li>").count()
    });
    rep.push(
        unr.is_none(),
        match unr {
            None => "not rendered: the page lists nothing".to_owned(),
            Some(n) => format!("not rendered: the page lists {n} item(s)"),
        },
    );

    let mut missing_ex = Vec::new();
    let mut n_ex = 0;
    for (cn, c) in ir
        .get("commands")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let Some(exs) = c
            .get("examples")
            .and_then(Value::as_object)
            .filter(|e| !e.is_empty())
        else {
            continue;
        };
        n_ex += 1;
        let card = tags.iter().position(|t| {
            t.name == "div"
                && t.attrs.get("class").map(String::as_str) == Some("card cmd")
                && t.attrs.get("data-ess-id") == Some(cn)
        });
        let shown = card
            .map(|i| {
                let s = tags[i].at;
                let e = tags[i + 1..]
                    .iter()
                    .find(|t| {
                        t.name == "div"
                            && t.attrs.get("class").map(String::as_str) == Some("card cmd")
                    })
                    .map_or(html.len(), |t| t.at);
                unescape(&html[s..e])
            })
            .unwrap_or_default();
        for (k, v) in exs {
            if !shown.contains(&format!("{k} = {v}")) {
                missing_ex.push(format!("{cn} {k}"));
            }
        }
    }
    if n_ex > 0 {
        rep.push(
            missing_ex.is_empty(),
            if missing_ex.is_empty() {
                format!("examples: every example of {n_ex} command(s) is shown on its card")
            } else {
                format!("examples: not shown: {missing_ex:?}")
            },
        );
    }
    rep
}

/// Adds the replay and demo-script checks: the page's simulation against a fresh bake.
pub fn check_simulation(rep: &mut Report, html: &str, fresh: &crate::sim::Baked) {
    let (_, scripts) = scan(html);
    let page: Value = scripts
        .get("ess-sim")
        .and_then(|t| serde_json::from_str(t).ok())
        .unwrap_or(Value::Null);
    for (label, key) in [
        ("synthesized suite", "failures"),
        ("authored scenarios", "authored_failures"),
    ] {
        let f = fresh
            .sim
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        rep.push(
            f.is_empty(),
            if f.is_empty() {
                format!("replay {label}: every expectation met")
            } else {
                format!(
                    "replay {label}: {} failed step(s): {:?}",
                    f.len(),
                    &f[..f.len().min(10)]
                )
            },
        );
    }
    rep.push(fresh.divergences.is_empty(), if fresh.divergences.is_empty() { format!("ESS lockstep: {} command(s) executed by ESS's interpreter, {} by the page's extensions, no divergence", fresh.by.0, fresh.by.1) } else { format!("ESS lockstep: {} divergence(s): {:?}", fresh.divergences.len(), &fresh.divergences[..fresh.divergences.len().min(10)]) });
    let demo = page.get("demo").cloned().unwrap_or(Value::Null);
    let pick = |s: &Value| {
        s.get("traces")
            .and_then(Value::as_array)
            .and_then(|ts| ts.iter().find(|t| t.get("id") == Some(&demo)).cloned())
    };
    match (pick(&page), pick(&fresh.sim)) {
        (Some(a), Some(b)) => {
            let same = a.get("steps") == b.get("steps");
            let n = a.get("steps").and_then(Value::as_array).map_or(0, Vec::len);
            rep.push(
                same,
                if same {
                    format!(
                        "demo script ({}): {n} steps equal a fresh bake",
                        text(&demo)
                    )
                } else {
                    format!(
                        "demo script ({}): the page's steps differ from a fresh bake",
                        text(&demo)
                    )
                },
            );
        }
        (None, None) if demo.is_null() => rep.push(true, "demo script: the page has no runs"),
        _ => rep.push(
            false,
            format!(
                "demo script: the page names demo `{}`, which a fresh bake does not carry",
                text(&demo)
            ),
        ),
    }
}

fn text(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned)
}
