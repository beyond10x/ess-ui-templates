//! Build-time simulation: the step scripts the page plays back.
//!
//! The page runs no interpreter. Everything it plays is baked here: one script per replayed
//! scenario, three seeded random walks over the whole system, and metrics over the walks. Seeded
//! runs choose among the commands each actor may send and build inputs only from values ESS gives:
//! identities of held instances, identities minted from the seed, enum variants, `example:` values
//! and literals the suites use for that command and field. They never invent text.
//!
//! Randomness is a xorshift generator seeded explicitly, so the same model, suites and seeds give
//! the same bytes.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ess_compiler::ir::{EssIr, ResolvedEffect, ResolvedInstance, ResolvedTypeRef};
use ess_primitives::node::Node;
use serde_json::{json, Map, Value};

use crate::interp::{Engine, Fields, Interp, World};
use crate::replay::{failures, jfields, jv, nv, replay_suite, Rec};

/// The xorshift generator the runs draw from.
pub struct Rand(u32);

impl Rand {
    /// Seeded.
    pub fn new(seed: u32) -> Self {
        let s = seed.wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9);
        Self(if s == 0 { 1 } else { s })
    }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            self.next() as usize % n
        }
    }
    fn chance(&mut self, p: f64) -> bool {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let limit = (p * 10000.0) as usize;
        self.below(10000) < limit
    }
    fn pick<'a, T>(&mut self, seq: &'a [T]) -> Option<&'a T> {
        if seq.is_empty() {
            None
        } else {
            let i = self.below(seq.len());
            seq.get(i)
        }
    }
}

fn transition_of(
    ir: &EssIr,
    interp: &Interp<'_>,
    command: Option<&str>,
    outcome: Option<&str>,
    entity: &str,
    before: &str,
    after: &str,
) -> Option<String> {
    if let (Some(c), Some(o)) = (command, outcome) {
        if let Some(cmd) = interp.command(c) {
            if let Some(o) = cmd.outcomes.iter().find(|x| x.name.to_string() == o) {
                if let Some(s) = &o.subject {
                    if let ResolvedEffect::Moves { transition } = &s.effect {
                        if s.entity.to_string() == entity
                            && transition.from.iter().any(|f| f.as_str() == before)
                            && transition.to.as_str() == after
                        {
                            return Some(transition.name.clone());
                        }
                    }
                }
            }
        }
    }
    let (_, e) = ir
        .entities()
        .iter()
        .find(|(k, _)| k.to_string() == entity)?;
    e.lifecycle
        .transitions
        .iter()
        .find(|t| t.from.iter().any(|f| f.as_str() == before) && t.to.as_str() == after)
        .map(|t| t.name.clone())
}

/// `[entity, identity, state before, state after, transition, fields after]` per changed instance.
pub fn deltas(
    ir: &EssIr,
    interp: &Interp<'_>,
    before: &World,
    after: &World,
    command: Option<&str>,
    outcome: Option<&str>,
) -> Vec<Value> {
    let mut out = Vec::new();
    let ents: BTreeSet<&String> = before.inst.keys().chain(after.inst.keys()).collect();
    for e in ents {
        let empty = BTreeMap::new();
        let b = before.inst.get(e).unwrap_or(&empty);
        let a = after.inst.get(e).unwrap_or(&empty);
        let ids: BTreeSet<&String> = b.keys().chain(a.keys()).collect();
        for id in ids {
            let (x, y) = (b.get(id), a.get(id));
            if x.map(|i| (&i.state, &i.fields)) == y.map(|i| (&i.state, &i.fields)) {
                continue;
            }
            let bs = x.map(|i| i.state.clone());
            let as_ = y.map(|i| i.state.clone());
            let tr = match (&bs, &as_) {
                (Some(p), Some(q)) if p != q => {
                    transition_of(ir, interp, command, outcome, e, p, q)
                }
                _ => None,
            };
            out.push(json!([e, id, bs, as_, tr, y.map(|i| jfields(&i.fields))]));
        }
    }
    out
}

fn subject_of(
    interp: &Interp<'_>,
    command: &str,
    input: &Fields,
) -> (Option<String>, Option<Value>) {
    let Some(cmd) = interp.command(command) else {
        return (None, None);
    };
    for o in &cmd.outcomes {
        if let Some(s) = &o.subject {
            let e = Some(s.entity.to_string());
            return match &s.instance {
                ResolvedInstance::Supplied { field } => (e, input.get(&field.name).map(jv)),
                ResolvedInstance::Observed { .. } => (e, None),
            };
        }
    }
    (None, None)
}

fn views_of(interp: &Interp<'_>, w: &World) -> BTreeMap<String, Value> {
    interp
        .ir
        .views()
        .keys()
        .map(|v| {
            let rows = interp
                .view_rows(w, &v.to_string(), None)
                .unwrap_or_default();
            (
                v.to_string(),
                Value::Array(rows.iter().map(|(k, r)| json!([k, jfields(r)])).collect()),
            )
        })
        .collect()
}

fn compact(m: Map<String, Value>) -> Value {
    Value::Object(
        m.into_iter()
            .filter(|(_, v)| match v {
                Value::Null => false,
                Value::Array(a) => !a.is_empty(),
                Value::Object(o) => !o.is_empty(),
                Value::String(s) => !s.is_empty(),
                _ => true,
            })
            .collect(),
    )
}

/// The component(s) that accept each command.
pub fn acceptors(ir: &EssIr) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (cn, c) in ir.components() {
        for x in &c.accepts {
            out.entry(x.to_string()).or_default().push(cn.to_string());
        }
    }
    out
}

fn events_json(rec: &Rec) -> Value {
    Value::Array(
        rec.events
            .iter()
            .map(|e| json!([e.event, jfields(&e.payload)]))
            .collect(),
    )
}

/// The step script for one replay trace. `labels`: identity → instance name.
pub fn script_of_trace(
    interp: &Interp<'_>,
    trace: &[Rec],
    acc: &BTreeMap<String, Vec<String>>,
    labels: &BTreeMap<String, String>,
    times: Option<&[String]>,
) -> (Vec<Value>, BTreeMap<String, String>) {
    let ir = interp.ir;
    let mut steps = Vec::new();
    let mut world = World::default();
    let mut views_before = views_of(interp, &world);
    let mut labels = labels.clone();
    let mut exec_i = 0;
    let mut i = 0;
    while i < trace.len() {
        let r = &trace[i];
        if r.step == "expect_no_event" {
            let mut j = i;
            let mut ok = true;
            let mut n = 0;
            while j < trace.len() && trace[j].step == "expect_no_event" {
                ok = ok && trace[j].ok == Some(true);
                n += 1;
                j += 1;
            }
            let mut m = Map::new();
            m.insert("k".into(), json!("expect_no_event"));
            m.insert("ok".into(), json!(ok));
            m.insert(
                "t".into(),
                json!(format!(
                    "no event among {n} checked: {}",
                    if ok {
                        "none emitted"
                    } else {
                        "one was emitted"
                    }
                )),
            );
            m.insert("n".into(), json!(n));
            steps.push(compact(m));
            i = j;
            continue;
        }
        let mut m = Map::new();
        m.insert("k".into(), json!(r.step));
        m.insert("ok".into(), json!(r.ok));
        m.insert("t".into(), json!(r.text));
        if r.unsupported {
            m.insert("u".into(), json!(1));
        }
        if r.step == "execute_command" {
            let c = r.command.clone().unwrap_or_default();
            m.insert("a".into(), json!(r.actor));
            m.insert("c".into(), json!(c));
            m.insert(
                "comp".into(),
                json!(acc.get(&c).cloned().unwrap_or_default()),
            );
            m.insert("o".into(), json!(r.outcome));
            m.insert("e".into(), json!(r.error));
            m.insert("ref".into(), json!(i32::from(r.refused)));
            m.insert("ev".into(), events_json(r));
            m.insert("inp".into(), jfields(&r.input));
            m.insert("note".into(), json!(r.note));
            m.insert("br".into(), json!(r.broken));
            m.insert("by".into(), json!(if r.by_ess { "ess" } else { "page" }));
            m.insert(
                "d".into(),
                Value::Array(deltas(
                    ir,
                    interp,
                    &world,
                    &r.world,
                    r.command.as_deref(),
                    r.outcome.as_deref(),
                )),
            );
            let (se, si) = subject_of(interp, &c, &r.input);
            m.insert("se".into(), json!(se));
            m.insert("si".into(), si.unwrap_or(Value::Null));
            if let Some(t) = times.and_then(|t| t.get(exec_i)) {
                m.insert("at".into(), json!(t));
            }
            exec_i += 1;
        }
        if r.step == "capture_instance" && r.ok == Some(true) {
            if let Some((_, val)) = r.text.split_once(" = ") {
                let inst = r
                    .raw
                    .get("instance")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                labels.insert(val.to_owned(), inst.clone());
                m.insert("cap".into(), json!([val, inst]));
            }
        }
        if let Some(v) = &r.view {
            m.insert("v".into(), json!(v));
        }
        if !r.world.same_rows(&world) {
            let now = views_of(interp, &r.world);
            let changed: Map<String, Value> = now
                .iter()
                .filter(|(v, rows)| views_before.get(*v) != Some(rows))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            m.insert("vs".into(), Value::Object(changed));
            views_before = now;
            world = r.world.clone();
        }
        steps.push(compact(m));
        i += 1;
    }
    (steps, labels)
}

/// `{(command, input field): literal values}` the suites use, in first-seen order.
pub fn suite_literals(suites: &[Option<&Value>]) -> BTreeMap<(String, String), Vec<Value>> {
    let mut out: BTreeMap<(String, String), Vec<Value>> = BTreeMap::new();
    for suite in suites.iter().flatten() {
        for sc in suite
            .get("scenarios")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(_, v)| v)
        {
            for st in sc
                .get("steps")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if st.get("step").and_then(Value::as_str) != Some("execute_command") {
                    continue;
                }
                let cmd = st
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                for (f, v) in st
                    .get("input")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flatten()
                {
                    if v.get("kind").and_then(Value::as_str) == Some("literal") {
                        let lst = out.entry((cmd.clone(), f.clone())).or_default();
                        let val = v.get("value").cloned().unwrap_or(Value::Null);
                        if !lst.contains(&val) {
                            lst.push(val);
                        }
                    }
                }
            }
        }
    }
    out
}

fn identity_types(ir: &EssIr) -> BTreeMap<String, String> {
    ir.entities()
        .iter()
        .filter_map(|(n, e)| {
            e.identity
                .type_ref
                .declared()
                .map(|t| (t.to_string(), n.to_string()))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn build_input(
    interp: &Interp<'_>,
    command: &str,
    w: &World,
    rng: &mut Rand,
    literals: &BTreeMap<(String, String), Vec<Value>>,
    id_types: &BTreeMap<String, String>,
    seed: u32,
    minted: &mut u64,
) -> Option<Fields> {
    let cmd = interp.command(command)?;
    let mut inp = Fields::new();
    for f in &cmd.input {
        let opt = f.type_ref.is_optional();
        if opt && rng.chance(0.5) {
            continue;
        }
        let base = f.type_ref.required();
        if let ResolvedTypeRef::Declared { name } = base {
            if let Some(entity) = id_types.get(&name.to_string()) {
                let held: Vec<Node> = w
                    .inst
                    .get(entity)
                    .map(|h| h.values().map(|i| i.id.clone()).collect())
                    .unwrap_or_default();
                if !held.is_empty() && !rng.chance(0.08) {
                    inp.insert(
                        f.name.clone(),
                        rng.pick(&held).cloned().unwrap_or(Node::Null),
                    );
                } else {
                    *minted += 1;
                    inp.insert(
                        f.name.clone(),
                        Node::Text(format!(
                            "00000000-0000-4000-9{:03x}-{:012x}",
                            seed % 4096,
                            *minted
                        )),
                    );
                }
                continue;
            }
        }
        let variants = interp.variants(&f.type_ref);
        let mut pool: Vec<Node> = literals
            .get(&(command.to_owned(), f.name.clone()))
            .into_iter()
            .flatten()
            .filter_map(nv)
            .collect();
        if let Some(ex) = cmd.examples.get(&f.name) {
            pool.push(ex.clone());
        }
        if !variants.is_empty() {
            let v = rng.pick(&variants).cloned().unwrap_or_default();
            inp.insert(f.name.clone(), Node::Text(v));
        } else if !pool.is_empty() {
            inp.insert(
                f.name.clone(),
                rng.pick(&pool).cloned().unwrap_or(Node::Null),
            );
        } else if !opt {
            return None;
        }
    }
    Some(inp)
}

/// A seeded random walk of `steps` commands over the whole system.
pub fn seeded_run<'a>(
    interp: &'a Interp<'a>,
    seed: u32,
    steps: usize,
    literals: &BTreeMap<(String, String), Vec<Value>>,
) -> (Vec<Rec>, Engine<'a>) {
    let ir = interp.ir;
    let mut rng = Rand::new(seed);
    let id_types = identity_types(ir);
    let mut engine = Engine::new(interp);
    let mut minted = 0u64;
    let mut pairs: Vec<(Option<String>, String)> = Vec::new();
    for (an, a) in ir.actors() {
        for c in &a.may {
            pairs.push((Some(an.to_string()), c.to_string()));
        }
    }
    if pairs.is_empty() {
        pairs = ir
            .commands()
            .keys()
            .map(|c| (None, c.to_string()))
            .collect();
    }
    let creates: BTreeSet<String> = ir
        .commands()
        .iter()
        .filter(|(_, c)| {
            c.outcomes.iter().any(|o| {
                o.subject
                    .as_ref()
                    .is_some_and(|s| matches!(s.effect, ResolvedEffect::Creates))
            })
        })
        .map(|(n, _)| n.to_string())
        .collect();
    let mut trace = Vec::new();
    for _ in 0..steps {
        let mut cands = Vec::new();
        for _ in 0..16 {
            let Some((an, c)) = rng.pick(&pairs).cloned() else {
                break;
            };
            let Some(inp) = build_input(
                interp,
                &c,
                &engine.world,
                &mut rng,
                literals,
                &id_types,
                seed,
                &mut minted,
            ) else {
                continue;
            };
            let Ok(st) = interp.step(&engine.world, &c, &inp, None, None) else {
                continue;
            };
            cands.push((an, c, inp, st));
        }
        if cands.is_empty() {
            break;
        }
        let held: usize = engine.world.inst.values().map(BTreeMap::len).sum();
        let valid: Vec<usize> = (0..cands.len())
            .filter(|&i| !cands[i].3.refused && cands[i].3.outcome.is_some())
            .collect();
        let advancing: Vec<usize> = valid
            .iter()
            .copied()
            .filter(|&i| !creates.contains(&cands[i].1) || held < 3)
            .collect();
        let refused: Vec<usize> = (0..cands.len()).filter(|i| !valid.contains(i)).collect();
        let all: Vec<usize> = (0..cands.len()).collect();
        let pick = if !advancing.is_empty() && rng.chance(0.8) {
            *rng.pick(&advancing).unwrap_or(&0)
        } else if !refused.is_empty() && rng.chance(0.6) {
            *rng.pick(&refused).unwrap_or(&0)
        } else {
            *rng.pick(if valid.is_empty() { &all } else { &valid })
                .unwrap_or(&0)
        };
        let (an, c, inp, _) = cands.swap_remove(pick);
        let Ok(st) = engine.execute(&c, &inp, None, None) else {
            continue;
        };
        let (delivered, invocations) = interp.deliver(&mut engine, &st.events, &BTreeMap::new(), 0);
        trace.push(Rec {
            step: "execute_command".into(),
            ok: Some(true),
            text: format!(
                "{c} → {}",
                st.outcome
                    .clone()
                    .unwrap_or_else(|| "no declared outcome".into())
            ),
            command: Some(c),
            actor: an,
            input: inp,
            outcome: st.outcome,
            error: st.error,
            events: st.events.into_iter().chain(delivered).collect(),
            invocations,
            note: st.note,
            broken: st.broken,
            refused: st.refused,
            open: st.open,
            world: engine.world.clone(),
            by_ess: st.by == crate::interp::By::Ess,
            ..Rec::default()
        });
    }
    (trace, engine)
}

/// Instance names for a run: `<Entity> <n>` in creation order.
pub fn run_labels(interp: &Interp<'_>, trace: &[Rec]) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen = World::default();
    for r in trace {
        for d in deltas(interp.ir, interp, &seen, &r.world, None, None) {
            let (e, id, b, a) = (
                d[0].as_str().unwrap_or(""),
                d[1].as_str().unwrap_or(""),
                &d[2],
                &d[3],
            );
            if b.is_null() && !a.is_null() && !labels.contains_key(id) {
                let n = count.entry(e.to_owned()).or_insert(0);
                *n += 1;
                labels.insert(
                    id.to_owned(),
                    format!("{}-{n}", e.rsplit('.').next().unwrap_or(e)),
                );
            }
        }
        seen = r.world.clone();
    }
    labels
}

/// The `at:` stamps of an authored scenario's timeline, read from its source file.
pub fn authored_times(dir: &Path, scenario: &str) -> (Option<Vec<String>>, Option<String>) {
    let short = scenario.rsplit('/').next().unwrap_or(scenario);
    let mut files = Vec::new();
    collect_yaml(dir, dir, &mut files);
    for (rel, path) in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let declares = text.lines().any(|l| {
            l.strip_prefix("scenario:")
                .is_some_and(|v| v.trim().trim_matches(|c| c == '\'' || c == '"') == short)
        });
        if !declares || !text.contains("ess-scenario") {
            continue;
        }
        let Some(start) = text.lines().position(|l| l.trim_end() == "timeline:") else {
            return (None, None);
        };
        let times = text
            .lines()
            .skip(start + 1)
            .filter_map(|l| {
                let t = l
                    .trim_start()
                    .strip_prefix('-')?
                    .trim_start()
                    .strip_prefix("at:")?;
                let t = t.trim().trim_matches(|c| c == '\'' || c == '"');
                let t: String = t
                    .chars()
                    .take_while(|c| !c.is_whitespace() && *c != '#' && *c != '\'' && *c != '"')
                    .collect();
                (!t.is_empty()).then_some(t)
            })
            .collect();
        return (Some(times), Some(rel));
    }
    (None, None)
}

fn collect_yaml(root: &Path, dir: &Path, out: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<_> = rd.filter_map(Result::ok).map(|e| e.path()).collect();
    children.sort();
    for c in children {
        let name = c.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if c.is_dir() {
            if !name.starts_with('.') && !matches!(name, "build" | "target" | "node_modules") {
                collect_yaml(root, &c, out);
            }
        } else if name.ends_with(".yaml") || name.ends_with(".yml") {
            out.push((
                c.strip_prefix(root)
                    .unwrap_or(&c)
                    .to_string_lossy()
                    .replace('\\', "/"),
                c,
            ));
        }
    }
}

/// Everything the page plays, and what the build checks it against.
pub struct Baked {
    /// The embedded simulation data.
    pub sim: Value,
    /// Authored traces by scenario name.
    pub authored: BTreeMap<String, Vec<Rec>>,
    /// Steps answered by ESS's interpreter, and by the page.
    pub by: (usize, usize),
    /// Where ESS and the page read a step differently.
    pub divergences: Vec<String>,
}

/// Bakes the demo script, scenario scripts, seeded runs and metrics.
#[allow(clippy::too_many_lines)]
pub fn bake(
    interp: &Interp<'_>,
    suite: Option<&Value>,
    authored: Option<&Value>,
    scenario_dir: Option<&Path>,
    demo_scenario: Option<&str>,
) -> Baked {
    let ir = interp.ir;
    let acc = acceptors(ir);
    let mut notes: Vec<String> = Vec::new();
    let mut traces: Vec<Value> = Vec::new();
    let (s_traces, tally, mut divergences) = suite
        .map(|s| replay_suite(interp, s, false))
        .unwrap_or_default();
    let (a_traces, a_tally, a_div) = authored
        .map(|s| replay_suite(interp, s, true))
        .unwrap_or_default();
    divergences.extend(a_div);
    let count = |t: &BTreeMap<String, Vec<Rec>>| {
        let ex: Vec<&Rec> = t
            .values()
            .flatten()
            .filter(|r| r.step == "execute_command")
            .collect();
        (
            ex.iter().filter(|r| r.by_ess).count(),
            ex.iter().filter(|r| !r.by_ess).count(),
        )
    };
    let (mut by_ess, mut by_page) = (0, 0);
    for t in [&s_traces, &a_traces] {
        let (e, p) = count(t);
        by_ess += e;
        by_page += p;
    }
    for (name, tr) in &a_traces {
        let (mut times, src) = scenario_dir
            .map(|d| authored_times(d, name))
            .unwrap_or((None, None));
        let n_exec = tr.iter().filter(|r| r.step == "execute_command").count();
        if let Some(t) = &times {
            if t.len() != n_exec {
                notes.push(format!(
                    "{name}: {} `at:` stamps for {n_exec} commands; times not shown",
                    t.len()
                ));
                times = None;
            }
        }
        let (steps, labels) = script_of_trace(interp, tr, &acc, &BTreeMap::new(), times.as_deref());
        traces.push(json!({"id": format!("authored:{name}"), "title": name.rsplit('/').next().unwrap_or(name), "group": "authored scenarios",
            "kind": "scenario", "origin": "authored", "verdict": verdict(&steps), "summary": purpose(authored, name), "steps": steps, "labels": labels, "times_from": src}));
    }
    for (name, tr) in &s_traces {
        let (steps, labels) = script_of_trace(interp, tr, &acc, &BTreeMap::new(), None);
        traces.push(json!({"id": format!("suite:{name}"), "title": name, "group": name.split('/').next().unwrap_or(name), "kind": "scenario",
            "origin": "synthesized", "verdict": verdict(&steps), "summary": purpose(suite, name), "steps": steps, "labels": labels}));
    }
    let literals = suite_literals(&[suite, authored]);
    let mut metrics = Vec::new();
    let declared: Vec<String> = ir
        .commands()
        .iter()
        .flat_map(|(c, cd)| cd.outcomes.iter().map(move |o| format!("{c}/{}", o.name)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let all_tr: BTreeSet<String> = ir
        .entities()
        .iter()
        .flat_map(|(e, ed)| {
            ed.lifecycle
                .transitions
                .iter()
                .map(move |t| format!("{e}#{}", t.name))
        })
        .collect();
    let mut reached_any = BTreeSet::new();
    for seed in [1u32, 2, 3] {
        let (tr, engine) = seeded_run(interp, seed, 60, &literals);
        divergences.extend(
            engine
                .divergences
                .iter()
                .map(|d| format!("run {seed}: {d}")),
        );
        by_ess += engine.by_ess;
        by_page += engine.by_page;
        if tr.is_empty() {
            notes.push(format!("seeded run {seed}: no command's input could be built from values ESS gives (held identities, minted identities, enum variants, examples, suite literals); pass --suite to use the suite's literals"));
        }
        let labels = run_labels(interp, &tr);
        let (steps, _) = script_of_trace(interp, &tr, &acc, &labels, None);
        let mut per_actor: BTreeMap<String, usize> = BTreeMap::new();
        let (mut ok, mut bad) = (0, 0);
        let mut outs = BTreeSet::new();
        let mut trs = BTreeSet::new();
        for (r, s) in tr.iter().zip(&steps) {
            *per_actor
                .entry(r.actor.clone().unwrap_or_else(|| "(no actor)".into()))
                .or_default() += 1;
            if r.refused || r.outcome.is_none() {
                bad += 1;
            } else {
                ok += 1;
            }
            if let (Some(c), Some(o)) = (&r.command, &r.outcome) {
                outs.insert(format!("{c}/{o}"));
            }
            for d in s.get("d").and_then(Value::as_array).into_iter().flatten() {
                if let Some(t) = d.get(4).and_then(Value::as_str) {
                    trs.insert(format!("{}#{t}", d[0].as_str().unwrap_or("")));
                }
            }
        }
        metrics.push(json!({"seed": seed, "steps": tr.len(), "per_actor": per_actor, "accepted": ok, "refused": bad,
            "outcomes": outs.len(), "outcomes_of": declared.len(), "transitions": trs.len(), "transitions_of": all_tr.len()}));
        reached_any.extend(outs);
        traces.push(json!({"id": format!("run:{seed}"), "title": format!("seeded run {seed}"), "group": "seeded runs", "kind": "run",
            "origin": "seeded", "summary": format!("A seeded random walk: {} command(s) the actors may send, inputs built from values ESS gives.", tr.len()), "steps": steps, "labels": labels}));
    }
    let unreached: Vec<&String> = declared
        .iter()
        .filter(|d| !reached_any.contains(*d))
        .collect();
    let mut demo = if a_traces.is_empty() {
        "run:1".to_owned()
    } else {
        traces[0]["id"].as_str().unwrap_or("run:1").to_owned()
    };
    if let Some(want) = demo_scenario {
        match traces
            .iter()
            .find(|t| t["kind"] == "scenario" && t["title"] == want)
        {
            Some(t) => demo = t["id"].as_str().unwrap_or("").to_owned(),
            None => notes.push(format!(
                "demo_scenario `{want}` is not an authored scenario; the first is the Demo"
            )),
        }
    }
    let tally_json = |t: &BTreeMap<String, [usize; 3]>| {
        Value::Object(t.iter().map(|(k, v)| (k.clone(), json!(v))).collect())
    };
    let sim = json!({
        "demo": demo, "traces": traces, "tally": tally_json(&tally), "authored_tally": tally_json(&a_tally),
        "failures": failures(&s_traces), "authored_failures": failures(&a_traces), "notes": notes,
        "metrics": metrics, "unreached": unreached,
        "engine": {"ess": by_ess, "page": by_page, "divergences": divergences},
    });
    Baked {
        sim,
        authored: a_traces,
        by: (by_ess, by_page),
        divergences,
    }
}

/// A scenario's verdict from its replayed steps: `failed` when an expectation failed,
/// `undetermined` when one could not be evaluated, else `met`.
pub fn verdict(steps: &[Value]) -> &'static str {
    if steps
        .iter()
        .any(|s| s.get("ok") == Some(&Value::Bool(false)))
    {
        "failed"
    } else if steps.iter().any(|s| s.get("u").is_some()) {
        "undetermined"
    } else {
        "met"
    }
}

/// The one-line purpose a suite gives a scenario, where it gives one.
fn purpose(suite: Option<&Value>, name: &str) -> Value {
    suite
        .and_then(|s| s.get("scenarios")?.get(name)?.get("purpose")?.as_str())
        .map(|p| p.lines().next().unwrap_or("").trim().to_owned())
        .filter(|p| !p.is_empty())
        .map_or(Value::Null, Value::String)
}
