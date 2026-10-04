//! Finds the authored file and line of every declaration by scanning the specification's YAML.
//!
//! The compiled IR carries no source locations, so this reads the files a specification is made
//! of, line by line, and records where each qualified name is declared. It never guesses: a
//! declaration whose line it cannot find is reported as unlocated. Nested declarations (states,
//! transitions, relations, outcomes) are found inside the indented block of their parent.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::model::{arr, g, obj, st, strs};

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn significant(line: &str) -> bool {
    let s = line.trim();
    !s.is_empty() && !s.starts_with('#')
}

/// The scalar after `key:` on a line, unquoted and without a comment.
fn scalar(rest: &str) -> Option<String> {
    let v = rest.trim();
    let v = v.split(" #").next().unwrap_or(v).trim();
    let v = v.trim_matches(|c| c == '\'' || c == '"');
    (!v.is_empty() && !v.contains(char::is_whitespace)).then(|| v.to_owned())
}

/// `(indent, dash, key, value)` of a `key: value` line.
fn key_line(line: &str) -> Option<(usize, bool, String, String)> {
    let ind = indent(line);
    let mut rest = &line[ind..];
    let dash = rest.starts_with("- ");
    if dash {
        rest = rest[2..].trim_start();
    }
    let (k, v) = rest.split_once(':')?;
    if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((ind, dash, k.to_owned(), scalar(v)?))
}

/// One file of the specification.
pub struct File {
    /// Its path relative to the specification directory.
    pub rel: String,
    lines: Vec<String>,
}

impl File {
    fn item_end(&self, i: usize) -> usize {
        let line = &self.lines[i];
        let mut col = i64::try_from(indent(line)).unwrap_or(0);
        if !line.trim_start().starts_with('-') {
            col = if col > 0 { (col - 2).max(0) } else { -1 };
        }
        for j in i + 1..self.lines.len() {
            if significant(&self.lines[j])
                && i64::try_from(indent(&self.lines[j])).unwrap_or(0) <= col
            {
                return j;
            }
        }
        self.lines.len()
    }

    fn find_key(&self, a: usize, b: usize, key: &str) -> Option<usize> {
        (a..b).find(|&k| {
            let t = self.lines[k].trim_start();
            let t = t.strip_prefix("- ").map_or(t, str::trim_start);
            t.strip_prefix(key).is_some_and(|r| {
                r.starts_with(':') && (r.len() == 1 || r[1..].starts_with(char::is_whitespace))
            })
        })
    }

    fn key_end(&self, k: usize, b: usize) -> usize {
        let mut col = indent(&self.lines[k]);
        if self.lines[k].trim_start().starts_with('-') {
            col += 2;
        }
        (k + 1..b)
            .find(|&j| significant(&self.lines[j]) && indent(&self.lines[j]) <= col)
            .unwrap_or(b)
    }

    fn named_items(&self, a: usize, b: usize) -> BTreeMap<String, usize> {
        let mut hits = Vec::new();
        for k in a..b {
            if let Some((ind, true, key, val)) = key_line(&self.lines[k]) {
                if key == "name" {
                    hits.push((ind, val, k));
                }
            }
        }
        let Some(col) = hits.iter().map(|h| h.0).min() else {
            return BTreeMap::new();
        };
        let mut found = BTreeMap::new();
        for (c, n, k) in hits {
            if c == col {
                found.entry(n).or_insert(k);
            }
        }
        found
    }
}

/// Where each declaration is written.
pub struct SourceIndex {
    files: Vec<File>,
    /// Files the specification lists that do not exist.
    pub missing_files: Vec<String>,
    decl: BTreeMap<(String, String), (usize, usize)>,
    dupes: BTreeSet<(String, String)>,
    /// The first `format:` value, with its file and line.
    pub format: Option<(String, String, usize)>,
}

impl SourceIndex {
    /// Indexes the specification in `dir`.
    pub fn new(dir: &Path) -> Self {
        let mut idx = Self {
            files: Vec::new(),
            missing_files: Vec::new(),
            decl: BTreeMap::new(),
            dupes: BTreeSet::new(),
            format: None,
        };
        let files = crate::spec::spec_files(dir).unwrap_or_default();
        for rel in files {
            let Ok(text) = std::fs::read_to_string(dir.join(&rel)) else {
                idx.missing_files.push(rel);
                continue;
            };
            let f = File {
                rel: rel.clone(),
                lines: text.split('\n').map(ToOwned::to_owned).collect(),
            };
            let fi = idx.files.len();
            for (i, line) in f.lines.iter().enumerate() {
                let Some((ind, _, key, val)) = key_line(line) else {
                    continue;
                };
                if !matches!(
                    key.as_str(),
                    "name" | "component" | "id" | "domain" | "system" | "format"
                ) {
                    continue;
                }
                if matches!(key.as_str(), "domain" | "system" | "format") && ind > 0 {
                    continue;
                }
                if key == "format" {
                    if idx.format.is_none() {
                        idx.format = Some((val, rel.clone(), i));
                    }
                    continue;
                }
                let k = (
                    if key == "domain" || key == "system" {
                        key
                    } else {
                        "item".to_owned()
                    },
                    val,
                );
                match idx.decl.entry(k) {
                    std::collections::btree_map::Entry::Occupied(e) => {
                        idx.dupes.insert(e.key().clone());
                    }
                    std::collections::btree_map::Entry::Vacant(e) => {
                        e.insert((fi, i));
                    }
                }
            }
            idx.files.push(f);
        }
        idx
    }

    fn top(&self, value: &str, key: &str) -> Option<(usize, usize)> {
        let k = (key.to_owned(), value.to_owned());
        if self.dupes.contains(&k) {
            return None;
        }
        self.decl.get(&k).copied()
    }

    fn nested(
        &self,
        parent: &str,
        section: &str,
        name: &str,
        scalar_list: bool,
    ) -> Option<(usize, usize)> {
        let (fi, i) = self.top(parent, "item")?;
        let f = &self.files[fi];
        let end = f.item_end(i);
        let k = f.find_key(i, end, section)?;
        let kend = f.key_end(k, end);
        if scalar_list {
            let head = &f.lines[k];
            if let (Some(a), Some(b)) = (head.find('['), head.rfind(']')) {
                let inner = &head[a + 1..b];
                return inner
                    .split(',')
                    .any(|x| x.trim().trim_matches(|c| c == '\'' || c == '"') == name)
                    .then_some((fi, k));
            }
            for j in k + 1..kend {
                let t = f.lines[j].trim_start();
                if let Some(v) = t.strip_prefix('-') {
                    if scalar(v).as_deref() == Some(name) {
                        return Some((fi, j));
                    }
                }
            }
            return None;
        }
        f.named_items(k + 1, kend).get(name).map(|&j| (fi, j))
    }

    fn workload(&self, name: &str) -> Option<(usize, usize)> {
        for (fi, f) in self.files.iter().enumerate() {
            for (i, line) in f.lines.iter().enumerate() {
                if line.trim() == "workloads:" {
                    let end = f.key_end(i, f.lines.len());
                    for j in i + 1..end {
                        let t = f.lines[j].trim();
                        if t.strip_suffix(':').is_some_and(|k| k == name)
                            || t.starts_with(&format!("{name}: #"))
                        {
                            return Some((fi, j));
                        }
                    }
                }
            }
        }
        None
    }

    /// `{(kind, id): (file, line)}` for every declaration the page tags, and the unlocated list.
    pub fn locate_all(
        &self,
        ir: &Value,
    ) -> (
        BTreeMap<(String, String), (String, usize)>,
        Vec<(String, String)>,
    ) {
        let mut loc = BTreeMap::new();
        let mut missing = Vec::new();
        let mut put = |kind: &str, ident: &str, hit: Option<(usize, usize)>| match hit {
            Some((fi, i)) => {
                loc.insert(
                    (kind.to_owned(), ident.to_owned()),
                    (self.files[fi].rel.clone(), i + 1),
                );
            }
            None => missing.push((kind.to_owned(), ident.to_owned())),
        };
        put(
            "system",
            st(ir, "system"),
            self.top(st(ir, "system"), "system"),
        );
        for d in obj(ir, "domains").keys() {
            put("domain", d, self.top(d, "domain"));
        }
        for c in obj(ir, "components").keys() {
            put("component", c, self.top(c, "item"));
        }
        let entities = obj(ir, "entities");
        for (kind, coll) in [
            ("actor", "actors"),
            ("entity", "entities"),
            ("command", "commands"),
            ("event", "events"),
            ("view", "views"),
            ("error", "errors"),
            ("type", "types"),
        ] {
            for n in obj(ir, coll).keys() {
                let mut hit = self.top(n, "item");
                if hit.is_none() && kind == "type" {
                    if let Some(owner) = n
                        .strip_suffix(".State")
                        .filter(|o| entities.contains_key(*o))
                    {
                        hit = self
                            .nested(owner, "lifecycle", "states", false)
                            .or_else(|| self.top(owner, "item"));
                    }
                }
                put(kind, n, hit);
            }
        }
        for (n, e) in entities {
            let lc = g(e, "lifecycle");
            for s in strs(lc, "states") {
                put(
                    "state",
                    &format!("{n}#{s}"),
                    self.nested(n, "states", &s, true),
                );
            }
            for t in arr(lc, "transitions") {
                let hit = self.nested(n, "transitions", st(t, "name"), false);
                for fr in strs(t, "from") {
                    put(
                        "transition",
                        &format!("{n}#{}:{fr}->{}", st(t, "name"), st(t, "to")),
                        hit,
                    );
                }
            }
            for r in arr(e, "relations") {
                put(
                    "relation",
                    &format!("{n}.{}->{}", st(r, "name"), st(r, "target")),
                    self.nested(n, "relations", st(r, "name"), false),
                );
            }
        }
        for (cn, c) in obj(ir, "commands") {
            for o in arr(c, "outcomes") {
                put(
                    "outcome",
                    &format!("{cn}/{}", st(o, "name")),
                    self.nested(cn, "outcomes", st(o, "name"), false),
                );
            }
        }
        for bn in obj(ir, "bindings").keys() {
            put("binding", bn, self.top(bn, "item"));
        }
        for wn in obj(ir, "workloads").keys() {
            put("workload", wn, self.workload(wn));
        }
        (loc, missing)
    }
}
