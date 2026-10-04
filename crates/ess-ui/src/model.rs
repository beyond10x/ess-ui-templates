//! Reads the compiled IR (`ess specify compile --format json`) into the model the page draws, and
//! records what the page cannot draw.
//!
//! Nothing here knows about a particular specification: only the vocabulary of the IR.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// A JSON null to borrow.
pub static NULL: Value = Value::Null;

/// `v[k]`, or null.
pub fn g<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&NULL)
}

/// `v[k]` as text, or "".
pub fn st<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// `v[k]` as a list, or empty.
pub fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `v[k]` as an object, or empty.
pub fn obj<'a>(v: &'a Value, k: &str) -> &'a Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    v.get(k)
        .and_then(Value::as_object)
        .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
}

/// A list of strings.
pub fn strs(v: &Value, k: &str) -> Vec<String> {
    arr(v, k)
        .iter()
        .filter_map(|x| x.as_str().map(ToOwned::to_owned))
        .collect()
}

/// A value as a reader sees it: text bare, everything else as JSON.
pub fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "none".into(),
        other => other.to_string(),
    }
}

/// The last segment of a qualified name.
pub fn short(q: &str) -> &str {
    q.rsplit('.').next().unwrap_or(q)
}

/// The `naming.display` of a declaration.
pub fn display(o: &Value) -> Option<String> {
    g(g(o, "naming"), "display").as_str().map(ToOwned::to_owned)
}

/// The `naming.summary` or `summary` of a declaration.
pub fn summary(o: &Value) -> String {
    let s = g(g(o, "naming"), "summary")
        .as_str()
        .or_else(|| g(o, "summary").as_str())
        .unwrap_or("");
    s.trim().to_owned()
}

/// A type reference as a label: `Money`, `optional<Email>`.
pub fn type_label(t: &Value) -> String {
    let Some(m) = t.as_object() else {
        return text(t);
    };
    let kind = st(t, "kind");
    if kind == "primitive" || kind == "declared" {
        return short(st(t, "name")).to_owned();
    }
    let inner: Vec<String> = m
        .iter()
        .filter(|(k, v)| *k != "kind" && v.is_object())
        .map(|(_, v)| type_label(v))
        .collect();
    if inner.is_empty() {
        t.to_string()
    } else {
        format!("{kind}<{}>", inner.join(", "))
    }
}

/// A predicate as text. The IR writes them as text; older shapes are mappings.
pub fn predicate_text(p: &Value) -> String {
    match p {
        Value::String(s) => s.clone(),
        Value::Object(m) => {
            let mut parts = Vec::new();
            for (field, test) in m {
                if let Some(t) = test.as_object() {
                    for (op, val) in t {
                        match (op.as_str(), val) {
                            ("any_of", Value::Array(vs)) => parts.push(format!(
                                "{field} ∈ {{{}}}",
                                vs.iter().map(text).collect::<Vec<_>>().join(", ")
                            )),
                            ("eq" | "equals", v) => parts.push(format!("{field} == {}", text(v))),
                            (op, v) => parts.push(format!("{field} {op} {v}")),
                        }
                    }
                } else {
                    parts.push(format!("{field} == {}", text(test)));
                }
            }
            parts.join(" ∧ ")
        }
        other => other.to_string(),
    }
}

/// `(field, variants)` a predicate selects, when it is an enum split.
pub fn predicate_variants(p: &Value) -> Option<(String, Vec<String>)> {
    match p {
        Value::String(s) => {
            let (l, r) = s.split_once("==")?;
            let (l, r) = (l.trim(), r.trim());
            let word =
                |x: &str| !x.is_empty() && x.chars().all(|c| c.is_alphanumeric() || c == '_');
            (word(l) && word(r)).then(|| (l.to_owned(), vec![r.to_owned()]))
        }
        Value::Object(m) if m.len() == 1 => {
            let (field, test) = m.iter().next()?;
            match test {
                Value::Object(t) if t.len() == 1 => {
                    let (op, val) = t.iter().next()?;
                    match (op.as_str(), val) {
                        ("any_of", Value::Array(vs)) => {
                            Some((field.clone(), vs.iter().map(text).collect()))
                        }
                        ("eq" | "equals", v) => Some((field.clone(), vec![text(v)])),
                        _ => None,
                    }
                }
                Value::String(s) => Some((field.clone(), vec![s.clone()])),
                _ => None,
            }
        }
        _ => None,
    }
}

/// An outcome's condition as text.
pub fn condition_text(c: &Value) -> String {
    let kind = st(c, "kind");
    match kind {
        "when" => format!("when {}", predicate_text(g(c, "predicate"))),
        "subject_predicate" => format!("subject_predicate {}", predicate_text(g(c, "predicate"))),
        "subject_state" => {
            let states = match g(c, "state") {
                Value::Array(a) => a.iter().map(text).collect::<Vec<_>>().join(" or "),
                other => text(other),
            };
            let p = g(c, "predicate");
            format!(
                "when the subject is {states}{}",
                if p.is_null() {
                    String::new()
                } else {
                    format!(" and {}", predicate_text(p))
                }
            )
        }
        "external" => format!("external: {}", st(c, "cause")),
        "existing_instance" => "when a record already carries the supplied identity".into(),
        "related" => {
            let via = g(c, "via");
            let e = short(st(c, "entity"));
            let row = format!("the {e} row {}.{} names", st(via, "from"), st(via, "field"));
            let test = g(c, "test");
            if test.as_str() == Some("absent") {
                return format!(
                    "when no {e} row carries {}.{}",
                    st(via, "from"),
                    st(via, "field")
                );
            }
            let guard = if g(c, "input").is_null() {
                String::new()
            } else {
                format!(" and {}", predicate_text(g(c, "input")))
            };
            match test.as_object() {
                Some(t) if t.len() == 1 && t.contains_key("holds") => {
                    let h = &t["holds"];
                    let p = if h.get("predicate").is_some() {
                        g(h, "predicate")
                    } else {
                        h
                    };
                    format!("when {row} holds {}{guard}", predicate_text(p))
                }
                _ => format!("when {row}: {test}{guard}"),
            }
        }
        _ => {
            let extra: Map<String, Value> = c
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter(|(k, _)| *k != "kind")
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect()
                })
                .unwrap_or_default();
            let k = if kind.is_empty() { "?" } else { kind };
            if extra.is_empty() {
                k.to_owned()
            } else {
                format!("{k} {}", Value::Object(extra))
            }
        }
    }
}

/// A value source as text.
pub fn value_text(v: &Value) -> String {
    if !v.is_object() {
        return v.to_string();
    }
    match st(v, "kind") {
        "input_field" => format!("input {}", st(v, "field")),
        "event_field" => format!("event {}", st(v, "field")),
        "literal" => format!("literal {}", text(g(v, "value"))),
        "increment" => format!("increment by {}", text(g(v, "by"))),
        kind => {
            let rest: Map<String, Value> = v
                .as_object()
                .map(|m| {
                    m.iter()
                        .filter(|(k, _)| *k != "kind" && *k != "type_ref")
                        .map(|(k, x)| (k.clone(), x.clone()))
                        .collect()
                })
                .unwrap_or_default();
            if rest.is_empty() {
                kind.to_owned()
            } else {
                format!("{kind} {}", Value::Object(rest))
            }
        }
    }
}

/// One assignment as text: `target ← source`, or just `target` when it copies the same input.
pub fn assign_text(a: &Value, always: bool) -> String {
    let v = g(a, "value");
    let tgt = a.get("target").and_then(Value::as_str).unwrap_or("?");
    if !always && st(v, "kind") == "input_field" && st(v, "field") == tgt {
        return tgt.to_owned();
    }
    format!("{tgt} ← {}", value_text(v))
}

/// Hues that read on both themes. A domain's colour comes from its name's hash, probing past
/// colours already taken, so it does not move when other domains change.
pub const PALETTE: [&str; 12] = [
    "#1f8fb3", "#c27a1a", "#7d63d9", "#23926a", "#c94a6d", "#3f7acc", "#a8890f", "#b8558a",
    "#4d8f2c", "#6574c9", "#b85a2f", "#2c8a85",
];

/// Each domain's colour.
pub fn domain_colours(domains: &[String]) -> BTreeMap<String, String> {
    let mut taken = BTreeSet::new();
    let mut out = BTreeMap::new();
    let mut sorted = domains.to_vec();
    sorted.sort();
    for d in sorted {
        let digest = Sha256::digest(d.as_bytes());
        // The hash modulo the palette size, read over the whole digest.
        let h = digest
            .iter()
            .fold(0usize, |acc, b| (acc * 256 + *b as usize) % PALETTE.len());
        for k in 0..PALETTE.len() {
            let c = (h + k) % PALETTE.len();
            if !taken.contains(&c) || taken.len() >= PALETTE.len() {
                taken.insert(c);
                out.insert(d.clone(), PALETTE[c].to_owned());
                break;
            }
        }
    }
    out
}

/// One edge of a lifecycle graph, as the path finder reads it.
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    /// Its element id.
    pub id: String,
    /// From state.
    pub from: String,
    /// To state.
    pub to: String,
}

/// For each terminal state, the edge ids of one shortest path from the initial state.
pub fn shortest_paths(initial: &str, terminal: &[String], edges: &[Edge]) -> Vec<Value> {
    let mut prev: BTreeMap<String, Option<usize>> = BTreeMap::new();
    prev.insert(initial.to_owned(), None);
    let mut q = VecDeque::from([initial.to_owned()]);
    while let Some(s) = q.pop_front() {
        for (i, e) in edges.iter().enumerate() {
            if e.from == s && !prev.contains_key(&e.to) {
                prev.insert(e.to.clone(), Some(i));
                q.push_back(e.to.clone());
            }
        }
    }
    let mut out = Vec::new();
    for t in terminal {
        if !prev.contains_key(t) || t == initial {
            continue;
        }
        let mut path = Vec::new();
        let mut s = t.clone();
        while let Some(Some(i)) = prev.get(&s) {
            path.push(edges[*i].id.clone());
            s = edges[*i].from.clone();
        }
        path.reverse();
        out.push(serde_json::json!({"to": t, "edges": path}));
    }
    out
}

/// Stable, unique element ids per declaration; the first element asking for `place` gets the id.
#[derive(Default)]
pub struct Anchors {
    /// `(kind, id)` → anchor.
    pub by_key: BTreeMap<(String, String), String>,
    used: BTreeSet<String>,
    placed: BTreeSet<String>,
    order: Vec<(String, String)>,
}

impl Anchors {
    /// The anchor of a declaration.
    pub fn of(&mut self, kind: &str, ident: &str) -> String {
        let key = (kind.to_owned(), ident.to_owned());
        if let Some(a) = self.by_key.get(&key) {
            return a.clone();
        }
        let clean: String = ident
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || "_.-".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let base = format!("{kind}-{clean}");
        let mut a = base.clone();
        let mut n = 2;
        while self.used.contains(&a) {
            a = format!("{base}-{n}");
            n += 1;
        }
        self.used.insert(a.clone());
        self.by_key.insert(key.clone(), a.clone());
        self.order.push(key);
        a
    }

    /// The anchor, once: `None` when an element already carries it as its id.
    pub fn place(&mut self, kind: &str, ident: &str) -> Option<String> {
        let a = self.of(kind, ident);
        self.placed.insert(a.clone()).then_some(a)
    }
}

/// What the page does not draw, collected while it is built.
#[derive(Default)]
pub struct Unrendered {
    /// Every item, in the order met.
    pub items: Vec<String>,
}

impl Unrendered {
    /// Notes one thing not rendered.
    pub fn add(&mut self, s: impl Into<String>) {
        self.items.push(s.into());
    }

    /// Every key of `o` the page does not know for `kind`.
    pub fn keys(&mut self, kind: &str, o: &Value, where_: &str) {
        let Some(m) = o.as_object() else { return };
        let known = known_keys(kind);
        for k in m.keys() {
            if !known.contains(&k.as_str()) {
                self.items.push(format!("{where_}: key `{k}` ({kind})"));
            }
        }
        if kind != "naming" {
            if let Some(n) = g(o, "naming").as_object() {
                for k in n.keys() {
                    if !known_keys("naming").contains(&k.as_str()) {
                        self.items.push(format!("{where_}: naming key `{k}`"));
                    }
                }
            }
        }
    }

    /// The sorted, distinct items.
    pub fn sorted(&self) -> Vec<String> {
        let s: BTreeSet<&String> = self.items.iter().collect();
        s.into_iter().cloned().collect()
    }
}

/// The keys the page renders or deliberately reads past, per IR object kind.
pub fn known_keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "top" => &[
            "system",
            "version",
            "naming",
            "summary",
            "domains",
            "types",
            "conversions",
            "entities",
            "commands",
            "events",
            "errors",
            "views",
            "actors",
            "bindings",
            "components",
            "workloads",
            "format",
            "preconditions",
        ],
        "domain" => &[
            "name", "naming", "types", "entities", "commands", "events", "errors", "views",
            "actors", "summary", "refs",
        ],
        "entity" => &[
            "name",
            "domain",
            "identity",
            "fields",
            "state_type",
            "lifecycle",
            "invariants",
            "naming",
            "relations",
        ],
        "lifecycle" => &["states", "initial", "terminal", "transitions"],
        "transition" => &["name", "from", "to"],
        "relation" => &["name", "kind", "target", "cardinality", "via"],
        "command" => &[
            "name",
            "domain",
            "input",
            "outcomes",
            "naming",
            "examples",
            "refs",
            "fixture_inputs",
            "response",
        ],
        "outcome" => &[
            "name",
            "condition",
            "subject",
            "test_strategy",
            "emits",
            "payload",
            "summary",
            "error",
            "sets",
            "complete_refusal",
            "error_payload",
            "refs",
            "refuses",
            "accepts_nothing",
            "returns",
            "decided_by_caller",
            "retains_result",
        ],
        "subject" => &["entity", "effect", "transition", "instance", "into"],
        "event" => &["name", "domain", "fields", "naming", "refs"],
        "view" => &[
            "name",
            "domain",
            "source",
            "fields",
            "filter",
            "order_by",
            "consistency",
            "assertion_style",
            "naming",
            "aggregation",
            "params",
            "shape",
        ],
        "actor" => &["name", "domain", "may", "naming", "attributes"],
        "component" => &[
            "name",
            "owns",
            "accepts",
            "publishes",
            "reached_by",
            "naming",
            "summary",
            "settings",
            "refs",
        ],
        "error" => &["name", "domain", "summary", "fields", "naming", "refs"],
        "type" => &["name", "body", "naming"],
        "binding" => &[
            "name",
            "event",
            "command",
            "mapping",
            "delivery",
            "failure",
            "escalation",
            "naming",
            "refs",
            "retry",
        ],
        "workload" => &["component", "replicas", "stateless", "requires"],
        "conversion" => &["from", "to", "because"],
        "naming" => &["wire", "display", "summary"],
        _ => &[],
    }
}

/// Condition kinds the page draws.
pub const KNOWN_CONDITIONS: &[&str] = &[
    "when",
    "otherwise",
    "wrong_state",
    "subject_predicate",
    "subject_state",
    "unknown_instance",
    "external",
    "existing_instance",
    "related",
];
/// Conditions that guard an outcome.
pub const GUARD_KINDS: &[&str] = &["when", "subject_predicate", "related"];
/// Conditions that refuse.
pub const REFUSAL_KINDS: &[&str] = &["wrong_state", "unknown_instance", "existing_instance"];
/// Effects the page draws.
pub const EFFECTS: &[&str] = &["moves", "creates", "updates", "deletes"];

/// The command › outcome that moves, creates, updates or deletes an entity.
#[derive(Debug, Clone, Serialize)]
pub struct Cause {
    /// The command.
    pub command: String,
    /// Its display name.
    pub display: Option<String>,
    /// The outcome.
    pub outcome: String,
    /// guarded, default, unconditional or other.
    pub kind: String,
    /// The condition as text.
    pub condition: String,
    /// Actors that may send the command.
    pub actors: Vec<String>,
    /// Components that accept it.
    pub components: Vec<String>,
    /// Events emitted, with payload assignments.
    pub emits: Vec<Value>,
    /// `sets:` as text.
    pub sets: Vec<String>,
    /// The outcome's summary.
    pub summary: String,
    /// The state a creation lands in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub into: Option<String>,
    /// Guards of the same command that refuse instead.
    pub refused_if: Vec<Value>,
}

/// The model the page draws.
pub struct Model<'a> {
    /// The IR.
    pub ir: &'a Value,
    /// Components.
    pub comps: &'a Map<String, Value>,
    /// Domains.
    pub domains: &'a Map<String, Value>,
    /// Entities.
    pub entities: &'a Map<String, Value>,
    /// Commands.
    pub commands: &'a Map<String, Value>,
    /// Events.
    pub events: &'a Map<String, Value>,
    /// Views.
    pub views: &'a Map<String, Value>,
    /// Actors.
    pub actors: &'a Map<String, Value>,
    /// Errors.
    pub errors: &'a Map<String, Value>,
    /// Types.
    pub types: &'a Map<String, Value>,
    /// Bindings.
    pub bindings: &'a Map<String, Value>,
    /// Workloads.
    pub workloads: &'a Map<String, Value>,
    /// Conversions.
    pub conversions: &'a [Value],
    /// Components in the order their entities relate.
    pub comp_order: Vec<String>,
    /// Domain → owning component.
    pub dom_owner: BTreeMap<String, String>,
    /// Command → actors that may send it.
    pub senders: BTreeMap<String, Vec<String>>,
    /// Command → components that accept it.
    pub acceptors: BTreeMap<String, Vec<String>>,
    /// Event → components that publish it.
    pub publishers: BTreeMap<String, Vec<String>>,
    /// (entity, transition) → causes.
    pub causes: BTreeMap<(String, String), Vec<Cause>>,
    /// Entity → creating causes.
    pub creators: BTreeMap<String, Vec<Cause>>,
    /// Entity → updating causes.
    pub updaters: BTreeMap<String, Vec<Cause>>,
    /// Entity → deleting causes.
    pub deleters: BTreeMap<String, Vec<Cause>>,
    /// Event → (command, outcome) that emit it.
    pub emitted_by: BTreeMap<String, Vec<(String, String)>>,
    /// Command → (entity, effect, transition) per outcome with a subject.
    pub effects_of: BTreeMap<String, Vec<(String, String, Option<String>)>>,
    /// Event → bindings.
    pub bind_by_event: BTreeMap<String, Vec<String>>,
    /// Command → bindings.
    pub bind_by_command: BTreeMap<String, Vec<String>>,
    /// Domain → colour.
    pub dom_colour: BTreeMap<String, String>,
    /// Element ids.
    pub anchors: RefCell<Anchors>,
    /// (kind, id) → added | changed.
    pub chg: BTreeMap<(String, String), String>,
    /// (kind, id) → (file, line).
    pub src: BTreeMap<(String, String), (String, usize)>,
    /// How a source line becomes a link.
    pub link: Box<dyn Fn(&str, usize) -> String + 'a>,
}

fn names(m: &Map<String, Value>) -> Vec<String> {
    m.keys().cloned().collect()
}

impl<'a> Model<'a> {
    /// The anchor of a declaration.
    pub fn a(&self, kind: &str, ident: &str) -> String {
        self.anchors.borrow_mut().of(kind, ident)
    }

    /// Builds the model, noting into `unr` what it will not draw.
    #[allow(clippy::too_many_lines)]
    pub fn build(ir: &'a Value, unr: &mut Unrendered) -> Self {
        unr.keys("top", ir, "IR");
        let comps = obj(ir, "components");
        let domains = obj(ir, "domains");
        let entities = obj(ir, "entities");
        let commands = obj(ir, "commands");
        let events = obj(ir, "events");
        let views = obj(ir, "views");
        let actors = obj(ir, "actors");
        let errors = obj(ir, "errors");
        let types = obj(ir, "types");
        let bindings = obj(ir, "bindings");
        let workloads = obj(ir, "workloads");
        let conversions = arr(ir, "conversions");
        for (n, o) in comps {
            unr.keys("component", o, &format!("component {n}"));
        }
        for (n, o) in domains {
            unr.keys("domain", o, &format!("domain {n}"));
        }
        for (n, o) in entities {
            unr.keys("entity", o, &format!("entity {n}"));
            let lc = g(o, "lifecycle");
            if lc.is_object() {
                unr.keys("lifecycle", lc, &format!("entity {n} lifecycle"));
                for t in arr(lc, "transitions") {
                    unr.keys(
                        "transition",
                        t,
                        &format!("entity {n} transition {}", st(t, "name")),
                    );
                }
            }
            for r in arr(o, "relations") {
                unr.keys(
                    "relation",
                    r,
                    &format!("entity {n} relation {}", st(r, "name")),
                );
            }
        }
        for (n, o) in commands {
            unr.keys("command", o, &format!("command {n}"));
            for oc in arr(o, "outcomes") {
                let on = st(oc, "name");
                unr.keys("outcome", oc, &format!("command {n} outcome {on}"));
                if g(oc, "subject").is_object() {
                    unr.keys(
                        "subject",
                        g(oc, "subject"),
                        &format!("command {n} outcome {on} subject"),
                    );
                }
                let ck = st(g(oc, "condition"), "kind");
                if !KNOWN_CONDITIONS.contains(&ck) {
                    unr.add(format!(
                        "command {n} outcome {on}: condition kind `{ck}` shown as raw text"
                    ));
                }
                let eff = st(g(oc, "subject"), "effect");
                if !eff.is_empty() && !EFFECTS.contains(&eff) {
                    unr.add(format!(
                        "command {n} outcome {on}: subject effect `{eff}` not drawn"
                    ));
                }
            }
        }
        for (kind, coll) in [
            ("event", events),
            ("view", views),
            ("actor", actors),
            ("error", errors),
            ("type", types),
            ("binding", bindings),
            ("workload", workloads),
        ] {
            for (n, o) in coll {
                unr.keys(kind, o, &format!("{kind} {n}"));
            }
        }
        for (i, c) in conversions.iter().enumerate() {
            unr.keys("conversion", c, &format!("conversion {}", i + 1));
        }

        let mut dom_owner = BTreeMap::new();
        for (cn, c) in comps {
            for d in strs(c, "owns") {
                dom_owner.entry(d).or_insert_with(|| cn.clone());
            }
        }
        let comp_names = names(comps);
        let mut succ: BTreeMap<String, BTreeSet<String>> = comp_names
            .iter()
            .map(|c| (c.clone(), BTreeSet::new()))
            .collect();
        for e in entities.values() {
            let a = dom_owner.get(st(e, "domain")).cloned();
            for r in arr(e, "relations") {
                let b = entities
                    .get(st(r, "target"))
                    .and_then(|t| dom_owner.get(st(t, "domain")))
                    .cloned();
                if let (Some(a), Some(b)) = (&a, b) {
                    if a != &b && succ.contains_key(a) && succ.contains_key(&b) {
                        succ.get_mut(a).unwrap().insert(b);
                    }
                }
            }
        }
        let mut indeg: BTreeMap<String, usize> =
            comp_names.iter().map(|c| (c.clone(), 0)).collect();
        for bs in succ.values() {
            for b in bs {
                *indeg.get_mut(b).unwrap() += 1;
            }
        }
        let idx = |c: &String| comp_names.iter().position(|x| x == c).unwrap_or(usize::MAX);
        let mut order: Vec<String> = Vec::new();
        let mut ready: Vec<String> = comp_names
            .iter()
            .filter(|c| indeg[*c] == 0)
            .cloned()
            .collect();
        while !ready.is_empty() {
            let c = ready.remove(0);
            order.push(c.clone());
            for b in &comp_names {
                if succ[&c].contains(b) {
                    let d = indeg.get_mut(b).unwrap();
                    *d -= 1;
                    if *d == 0 {
                        ready.push(b.clone());
                    }
                }
            }
            ready.sort_by_key(idx);
        }
        for c in &comp_names {
            if !order.contains(c) {
                order.push(c.clone());
            }
        }

        let mut senders: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut acceptors: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut publishers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (an, a) in actors {
            for c in strs(a, "may") {
                if !commands.contains_key(&c) {
                    unr.add(format!(
                        "actor {an} may `{c}`, which is not a command in the IR"
                    ));
                }
                senders.entry(c).or_default().push(an.clone());
            }
        }
        for (cn, c) in comps {
            for x in strs(c, "accepts") {
                acceptors.entry(x).or_default().push(cn.clone());
            }
            for x in strs(c, "publishes") {
                publishers.entry(x).or_default().push(cn.clone());
            }
        }

        let mut causes: BTreeMap<(String, String), Vec<Cause>> = BTreeMap::new();
        let mut creators: BTreeMap<String, Vec<Cause>> = BTreeMap::new();
        let mut updaters: BTreeMap<String, Vec<Cause>> = BTreeMap::new();
        let mut deleters: BTreeMap<String, Vec<Cause>> = BTreeMap::new();
        let mut emitted_by: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut refusing: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        let mut effects_of: BTreeMap<String, Vec<(String, String, Option<String>)>> =
            BTreeMap::new();
        for (cn, c) in commands {
            let ocs = arr(c, "outcomes");
            let branching = ocs
                .iter()
                .any(|o| GUARD_KINDS.contains(&st(g(o, "condition"), "kind")));
            for oc in ocs {
                let cond = g(oc, "condition");
                let ck = st(cond, "kind");
                for ev in strs(oc, "emits") {
                    emitted_by
                        .entry(ev)
                        .or_default()
                        .push((cn.clone(), st(oc, "name").to_owned()));
                }
                let subj = g(oc, "subject");
                if !subj.is_object() {
                    if GUARD_KINDS.contains(&ck) {
                        let ctext = if ck == "related" {
                            condition_text(cond)
                        } else {
                            predicate_text(g(cond, "predicate"))
                        };
                        refusing.entry(cn.clone()).or_default().push(serde_json::json!({"outcome": st(oc, "name"), "condition": ctext, "error": g(oc, "error")}));
                    }
                    continue;
                }
                let payload: BTreeMap<String, Vec<String>> = arr(oc, "payload")
                    .iter()
                    .map(|p| {
                        (
                            st(p, "event").to_owned(),
                            arr(p, "fields")
                                .iter()
                                .map(|f| assign_text(f, false))
                                .collect(),
                        )
                    })
                    .collect();
                let kind = if GUARD_KINDS.contains(&ck) {
                    "guarded"
                } else if ck == "otherwise" && branching {
                    "default"
                } else if ck == "otherwise" {
                    "unconditional"
                } else {
                    "other"
                };
                let ent = st(subj, "entity").to_owned();
                let eff = st(subj, "effect").to_owned();
                let mut cause = Cause {
                    command: cn.clone(),
                    display: display(c),
                    outcome: st(oc, "name").to_owned(),
                    kind: kind.to_owned(),
                    condition: condition_text(cond),
                    actors: senders.get(cn).cloned().unwrap_or_default(),
                    components: acceptors.get(cn).cloned().unwrap_or_default(),
                    emits: strs(oc, "emits").into_iter().map(|ev| serde_json::json!({"event": ev, "fields": payload.get(&ev).cloned().unwrap_or_default()})).collect(),
                    sets: arr(oc, "sets").iter().map(|s| assign_text(s, true)).collect(),
                    summary: st(oc, "summary").trim().to_owned(),
                    into: None,
                    refused_if: Vec::new(),
                };
                let tname = g(subj, "transition")
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
                effects_of.entry(cn.clone()).or_default().push((
                    ent.clone(),
                    eff.clone(),
                    tname.clone(),
                ));
                match eff.as_str() {
                    "moves" if tname.is_some() => {
                        causes.entry((ent, tname.unwrap())).or_default().push(cause)
                    }
                    "creates" => {
                        let into = g(subj, "into").as_str().map(ToOwned::to_owned).or_else(|| {
                            entities
                                .get(&ent)
                                .map(|e| st(g(e, "lifecycle"), "initial").to_owned())
                        });
                        cause.into = into;
                        creators.entry(ent).or_default().push(cause);
                    }
                    "updates" => updaters.entry(ent).or_default().push(cause),
                    "deletes" => deleters.entry(ent).or_default().push(cause),
                    _ => {}
                }
            }
        }
        for cs in causes.values_mut() {
            for c in cs {
                c.refused_if = refusing.get(&c.command).cloned().unwrap_or_default();
            }
        }
        let mut bind_by_event: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut bind_by_command: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (bn, b) in bindings {
            bind_by_event
                .entry(st(b, "event").to_owned())
                .or_default()
                .push(bn.clone());
            bind_by_command
                .entry(st(b, "command").to_owned())
                .or_default()
                .push(bn.clone());
        }
        Self {
            ir,
            comps,
            domains,
            entities,
            commands,
            events,
            views,
            actors,
            errors,
            types,
            bindings,
            workloads,
            conversions,
            comp_order: order,
            dom_owner,
            senders,
            acceptors,
            publishers,
            causes,
            creators,
            updaters,
            deleters,
            emitted_by,
            effects_of,
            bind_by_event,
            bind_by_command,
            dom_colour: domain_colours(&names(domains)),
            anchors: RefCell::new(Anchors::default()),
            chg: BTreeMap::new(),
            src: BTreeMap::new(),
            link: Box::new(|f, l| format!("{f}#L{l}")),
        }
    }
}
