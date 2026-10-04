//! Replays conformance scenarios (`ess-conformance` suite JSON) through the [`Engine`]: every step
//! the page plays, and the tally of expectations met.

use std::collections::BTreeMap;

use ess_primitives::node::Node;
use serde_json::{json, Map, Value};

use crate::interp::{Emitted, Engine, Fields, Interp, Invocation, Undetermined, World};

/// Step kinds that are expectations rather than actions.
pub const EXPECTATIONS: &[&str] = &[
    "expect_outcome",
    "expect_error",
    "expect_no_error",
    "expect_event",
    "eventually_event",
    "expect_no_event",
    "expect_no_events",
    "expect_view",
    "eventually_view",
    "expect_complete_subject_unchanged",
    "expect_invocation",
    "expect_subject_absent",
    "expect_not_granted",
];

/// One replayed step.
#[derive(Debug, Clone, Default)]
pub struct Rec {
    /// The step kind.
    pub step: String,
    /// Met (`Some(true)`), failed (`Some(false)`) or not evaluable (`None`).
    pub ok: Option<bool>,
    /// What happened, in one line.
    pub text: String,
    /// The step as the suite wrote it.
    pub raw: Value,
    /// The command, for `execute_command`.
    pub command: Option<String>,
    /// The actor.
    pub actor: Option<String>,
    /// The resolved input.
    pub input: Fields,
    /// The outcome.
    pub outcome: Option<String>,
    /// The error.
    pub error: Option<String>,
    /// Events emitted, delivered events after them.
    pub events: Vec<Emitted>,
    /// Binding invocations.
    pub invocations: Vec<Invocation>,
    /// The page's note.
    pub note: Option<String>,
    /// A broken invariant.
    pub broken: Option<String>,
    /// Whether it refused.
    pub refused: bool,
    /// Answers left open.
    pub open: Vec<(Option<String>, Option<String>)>,
    /// Not evaluated.
    pub unsupported: bool,
    /// The view the step reads.
    pub view: Option<String>,
    /// The world after the step.
    pub world: World,
    /// Answered by ESS's interpreter.
    pub by_ess: bool,
}

#[derive(Default)]
struct Last {
    command: Option<String>,
    outcome: Option<String>,
    error: Option<String>,
    events: Vec<Emitted>,
    delivered: Vec<Emitted>,
    invocations: Vec<Invocation>,
    open: Vec<(Option<String>, Option<String>)>,
    fields: Fields,
    not_granted: bool,
}

struct Ctx<'a, 'ir> {
    interp: &'a Interp<'ir>,
    engine: Engine<'a>,
    captures: BTreeMap<String, Value>,
    snapshots: BTreeMap<String, (Map<String, Value>, Option<Value>)>,
    forced: BTreeMap<String, String>,
    history: Vec<Emitted>,
    last: Last,
    view_params: BTreeMap<String, Option<Fields>>,
    view_snapshots: BTreeMap<String, Vec<(String, Fields)>>,
}

/// A node as JSON.
pub fn jv(n: &Node) -> Value {
    serde_json::to_value(n).unwrap_or(Value::Null)
}

/// JSON as a node.
pub fn nv(v: &Value) -> Option<Node> {
    if v.is_null() {
        return None;
    }
    serde_json::from_value(v.clone()).ok()
}

/// Fields as a JSON object.
pub fn jfields(f: &Fields) -> Value {
    Value::Object(f.iter().map(|(k, v)| (k.clone(), jv(v))).collect())
}

/// Equality as a scenario means it: numbers by value, every member of objects and arrays.
pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| values_equal(v, w)))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(v, w)| values_equal(v, w))
        }
        _ => a == b,
    }
}

/// The value at a dotted path of a JSON object; `count` of a text or list is its length.
pub fn lookup(v: &Value, path: &str) -> Option<Value> {
    let mut cur = v.clone();
    for part in path.split('.') {
        if part == "count" {
            if let Value::String(s) = &cur {
                cur = json!(s.chars().count());
                continue;
            }
            if let Value::Array(a) = &cur {
                cur = json!(a.len());
                continue;
            }
        }
        cur = cur.as_object()?.get(part)?.clone();
        if cur.is_null() {
            return None;
        }
    }
    Some(cur)
}

impl Ctx<'_, '_> {
    fn resolve(&self, v: &Value) -> Result<Value, Undetermined> {
        let kind = v.get("kind").and_then(Value::as_str).unwrap_or("");
        Ok(match kind {
            "members" => {
                let mut m = Map::new();
                for (k, x) in v
                    .get("members")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flatten()
                {
                    m.insert(k.clone(), self.resolve(x)?);
                }
                Value::Object(m)
            }
            "literal" => v.get("value").cloned().unwrap_or(Value::Null),
            "instance" => v
                .get("instance")
                .and_then(Value::as_str)
                .and_then(|i| self.captures.get(i))
                .cloned()
                .unwrap_or(Value::Null),
            "observed" => {
                let event = v.get("event").and_then(Value::as_str).unwrap_or("");
                let field = v.get("field").and_then(Value::as_str).unwrap_or("");
                self.history
                    .iter()
                    .find(|e| e.event == event)
                    .and_then(|e| e.payload.get(field))
                    .map_or(Value::Null, jv)
            }
            other => return Err(Undetermined(format!("scenario value `{other}`"))),
        })
    }

    fn resolve_map(&self, v: Option<&Value>) -> Result<Map<String, Value>, Undetermined> {
        let mut out = Map::new();
        for (k, x) in v.and_then(Value::as_object).into_iter().flatten() {
            out.insert(k.clone(), self.resolve(x)?);
        }
        Ok(out)
    }

    fn view_params(&self, st: &Value) -> Result<Option<Fields>, Undetermined> {
        let view = st.get("view").and_then(Value::as_str).unwrap_or("");
        if st.get("step").and_then(Value::as_str) == Some("query_view")
            || st.get("params").is_some()
        {
            let m = self.resolve_map(st.get("params"))?;
            return Ok(Some(
                m.iter()
                    .filter_map(|(k, v)| nv(v).map(|n| (k.clone(), n)))
                    .collect(),
            ));
        }
        Ok(self.view_params.get(view).cloned().flatten())
    }

    fn rows(
        &self,
        view: &str,
        params: Option<&Fields>,
    ) -> Result<Vec<(String, Fields)>, Undetermined> {
        self.interp.view_rows(&self.engine.world, view, params)
    }
}

fn shape_ok(
    event: &Emitted,
    shape: Option<&Value>,
    skipped: &mut Vec<String>,
) -> (Option<bool>, Option<String>) {
    let payload = jfields(&event.payload);
    let mut ok = Some(true);
    let mut why = None;
    for (field, want) in shape.and_then(Value::as_object).into_iter().flatten() {
        let got = lookup(&payload, field);
        let Some(v) = got else {
            if want.get("optional").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            match unknown(event, field, skipped) {
                Some(false) => return (Some(false), Some(format!("field `{field}` is absent"))),
                None => {
                    if ok == Some(true) {
                        ok = None;
                        why = Some(format!(
                            "the model does not determine `{field}`; an implementation assigns it"
                        ));
                    }
                }
                Some(true) => {}
            }
            continue;
        };
        let kind = want.get("kind").and_then(Value::as_str).unwrap_or("");
        let bad = match kind {
            "uuid" => !v.as_str().is_some_and(|s| {
                s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            }),
            "string" | "timestamp" => !v.is_string(),
            "integer" | "decimal" => !v.is_number(),
            _ => false,
        };
        if bad {
            return (
                Some(false),
                Some(format!("`{field}` = {v} is not a {kind}")),
            );
        }
        if want.get("holds").and_then(Value::as_str) == Some("enum") {
            if let Some(vs) = want.get("variants").and_then(Value::as_array) {
                if !vs.contains(&v) {
                    return (
                        Some(false),
                        Some(format!("`{field}` = {v} is not a declared variant")),
                    );
                }
            }
        }
    }
    (ok, why)
}

/// `Some(false)`: absent; `None`: open (left to the implementation); `Some(true)`: skipped by name.
fn unknown(event: &Emitted, field: &str, skipped: &mut Vec<String>) -> Option<bool> {
    let top = field.split('.').next().unwrap_or(field).to_owned();
    if event.uncalled.contains(&top) {
        skipped.push(top);
        return Some(true);
    }
    if event.open.contains(&top) {
        return None;
    }
    Some(false)
}

fn row_matches(row: &Value, fields: &Map<String, Value>) -> bool {
    fields
        .iter()
        .all(|(name, want)| match (lookup(row, name), want.is_null()) {
            (got, true) => got.is_none(),
            (Some(got), false) => values_equal(&got, want),
            (None, false) => false,
        })
}

/// Replays one scenario. `authored`: a step left open meets an expectation of any answer it leaves
/// open, as ESS synthesizes a step only where one branch is selected.
#[allow(clippy::too_many_lines)]
pub fn replay<'a>(
    interp: &'a Interp<'a>,
    scenario: &Value,
    authored: bool,
) -> (Vec<Rec>, Engine<'a>) {
    let mut ctx = Ctx {
        interp,
        engine: Engine::new(interp),
        captures: BTreeMap::new(),
        snapshots: BTreeMap::new(),
        forced: BTreeMap::new(),
        history: Vec::new(),
        last: Last::default(),
        view_params: BTreeMap::new(),
        view_snapshots: BTreeMap::new(),
    };
    let steps: Vec<Value> = scenario
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut trace = Vec::new();
    for (n, st) in steps.iter().enumerate() {
        let kind = st
            .get("step")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut rec = Rec {
            step: kind.clone(),
            raw: st.clone(),
            ..Rec::default()
        };
        let result = step(&mut ctx, &mut rec, st, steps.get(n + 1), authored);
        if let Err(u) = result {
            rec.ok = Some(false);
            rec.text = format!("undetermined: {u}");
        }
        rec.world = ctx.engine.world.clone();
        trace.push(rec);
    }
    (trace, ctx.engine)
}

fn s<'v>(v: &'v Value, k: &str) -> &'v str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

#[allow(clippy::too_many_lines)]
fn step(
    ctx: &mut Ctx<'_, '_>,
    rec: &mut Rec,
    st: &Value,
    next: Option<&Value>,
    authored: bool,
) -> Result<(), Undetermined> {
    let kind = rec.step.clone();
    let ir = ctx.interp.ir;
    match kind.as_str() {
        "execute_command" => {
            let command = s(st, "command").to_owned();
            rec.command = Some(command.clone());
            rec.actor = st
                .get("actor")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let raw = ctx.resolve_map(st.get("input"))?;
            let input: Fields = raw
                .iter()
                .filter_map(|(k, v)| nv(v).map(|n| (k.clone(), n)))
                .collect();
            rec.input = input.clone();
            if let Some(actor) = &rec.actor {
                let granted = ir.actors().iter().any(|(k, a)| {
                    k.to_string() == *actor && a.may.iter().any(|c| c.to_string() == command)
                });
                if !granted {
                    ctx.last = Last {
                        command: Some(command.clone()),
                        not_granted: true,
                        ..Last::default()
                    };
                    rec.ok = Some(true);
                    rec.refused = true;
                    rec.text = format!("{command} → not granted to {actor}");
                    rec.note = Some(format!("{actor} may not send {command}"));
                    return Ok(());
                }
            }
            let mut forced = ctx.forced.get(&command).cloned();
            if forced.is_none() {
                if let Some(nx) = next.filter(|nx| s(nx, "step") == "expect_outcome") {
                    let want = nx.get("outcome").cloned().unwrap_or(Value::Null);
                    if s(&want, "command") == command {
                        // An authored act that expects an external branch states the external answer.
                        if let Some(cmd) = ctx.interp.command(&command) {
                            if cmd.outcomes.iter().any(|o| o.name.to_string() == s(&want, "outcome") && matches!(o.condition, ess_compiler::ir::ResolvedCondition::External { .. } | ess_compiler::ir::ResolvedCondition::ExternalWhen { .. })) {
                                forced = Some(s(&want, "outcome").to_owned());
                            }
                        }
                    }
                }
            }
            let caller: Option<Fields> = st.get("caller").and_then(Value::as_object).map(|m| {
                m.iter()
                    .filter_map(|(k, v)| nv(v).map(|n| (k.clone(), n)))
                    .collect()
            });
            let before = ctx.engine.world.clone();
            let step = ctx
                .engine
                .execute(&command, &input, forced.as_deref(), caller.as_ref())?;
            let forced_all = ctx.forced.clone();
            let (delivered, invocations) =
                ctx.interp
                    .deliver(&mut ctx.engine, &step.events, &forced_all, 0);
            ctx.history.extend(step.events.iter().cloned());
            ctx.history.extend(delivered.iter().cloned());
            let fields = if step.error.is_some() {
                ctx.interp.error_fields(
                    &before,
                    &command,
                    step.outcome.as_deref().unwrap_or(""),
                    &input,
                    caller.as_ref(),
                )
            } else {
                Fields::new()
            };
            ctx.last = Last {
                command: Some(command.clone()),
                outcome: step.outcome.clone(),
                error: step.error.clone(),
                events: step.events.clone(),
                delivered: delivered.clone(),
                invocations: invocations.clone(),
                open: step.open.clone(),
                fields,
                not_granted: false,
            };
            rec.outcome = step.outcome.clone();
            rec.error = step.error.clone();
            rec.events = step.events.iter().cloned().chain(delivered).collect();
            rec.invocations = invocations;
            rec.note = step.note.clone();
            rec.broken = step.broken.clone();
            rec.refused = step.refused;
            rec.by_ess = step.by == crate::interp::By::Ess;
            rec.ok = Some(true);
            rec.text = format!(
                "{command} → {}",
                step.outcome
                    .clone()
                    .unwrap_or_else(|| "no declared outcome".into())
            );
            if !step.open.is_empty() {
                rec.open = step.open.clone();
                if !authored {
                    rec.ok = Some(false);
                    rec.text
                        .push_str(": left open, where a synthesized step selects one branch");
                }
            }
        }
        "expect_outcome" | "expect_error" | "expect_no_error" => {
            let last = &ctx.last;
            let answers: Vec<(Option<String>, Option<String>)> =
                if authored && !last.open.is_empty() {
                    last.open.clone()
                } else {
                    vec![(last.outcome.clone(), last.error.clone())]
                };
            let (ok, mut text) = match kind.as_str() {
                "expect_outcome" => {
                    let want = st.get("outcome").cloned().unwrap_or(Value::Null);
                    let ok = last.command.as_deref() == Some(s(&want, "command"))
                        && answers
                            .iter()
                            .any(|(o, _)| o.as_deref() == Some(s(&want, "outcome")));
                    (
                        ok,
                        format!(
                            "outcome {}: got {}",
                            s(&want, "outcome"),
                            last.outcome.clone().unwrap_or_else(|| if last.not_granted {
                                "not granted".into()
                            } else {
                                "none".into()
                            })
                        ),
                    )
                }
                "expect_error" => {
                    let want = s(st, "error");
                    let mut ok = answers.iter().any(|(_, e)| e.as_deref() == Some(want));
                    let mut text = format!(
                        "error {want}: got {}",
                        last.error.clone().unwrap_or_else(|| "none".into())
                    );
                    if let Some(wf) = st
                        .get("fields")
                        .and_then(Value::as_object)
                        .filter(|m| !m.is_empty())
                    {
                        if last.error.as_deref() == Some(want) {
                            let got = jfields(&last.fields);
                            let bad: Vec<&String> = wf
                                .iter()
                                .filter(|(k, v)| {
                                    !got.get(k.as_str()).is_some_and(|g| values_equal(g, v))
                                })
                                .map(|(k, _)| k)
                                .collect();
                            ok = ok && bad.is_empty();
                            text.push_str(&format!(", fields {}", Value::Object(wf.clone())));
                            if !bad.is_empty() {
                                text.push_str(&format!(": got {got}"));
                            }
                        }
                    }
                    (ok, text)
                }
                _ => (
                    answers.iter().any(|(_, e)| e.is_none()) && !last.not_granted,
                    format!(
                        "no error: got {}",
                        last.error.clone().unwrap_or_else(|| "none".into())
                    ),
                ),
            };
            if authored && !last.open.is_empty() {
                text.push_str(&format!(
                    " (the model leaves {} open)",
                    answers
                        .iter()
                        .map(|(o, _)| o.clone().unwrap_or_else(|| "none".into()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            rec.ok = Some(ok);
            rec.text = text;
        }
        "expect_not_granted" => {
            rec.ok = Some(ctx.last.not_granted);
            rec.text = format!(
                "not granted: {}",
                if ctx.last.not_granted {
                    "refused before the command ran"
                } else {
                    "the command ran"
                }
            );
        }
        "expect_event" | "eventually_event" => {
            let event = s(st, "event");
            let pool: Vec<&Emitted> = ctx
                .last
                .events
                .iter()
                .chain(if kind == "eventually_event" {
                    ctx.last.delivered.iter()
                } else {
                    [].iter()
                })
                .filter(|e| e.event == event)
                .collect();
            let mut skipped = Vec::new();
            let (mut ok, mut why) = match pool.last() {
                Some(hit) => shape_ok(hit, st.get("shape"), &mut skipped),
                None => (Some(false), Some("not emitted".into())),
            };
            if ok != Some(false) {
                if let Some(hit) = pool.last() {
                    let payload = jfields(&hit.payload);
                    for (field, v) in st
                        .get("payload")
                        .and_then(Value::as_object)
                        .into_iter()
                        .flatten()
                    {
                        let expected = if v.get("kind").is_some() && v.is_object() {
                            ctx.resolve(v)?
                        } else {
                            v.clone()
                        };
                        match lookup(&payload, field) {
                            None => match unknown(hit, field, &mut skipped) {
                                Some(false) => {
                                    ok = Some(false);
                                    why = Some(format!("field `{field}` is absent"));
                                    break;
                                }
                                None if ok == Some(true) => {
                                    ok = None;
                                    why = Some(format!("the model does not determine `{field}`; an implementation assigns it"));
                                }
                                _ => {}
                            },
                            Some(got) if !values_equal(&got, &expected) => {
                                ok = Some(false);
                                why = Some(format!("`{field}` = {got}, expected {expected}"));
                                break;
                            }
                            Some(_) => {}
                        }
                    }
                }
            }
            skipped.sort();
            skipped.dedup();
            rec.text = format!(
                "event {event}: {}",
                if ok == Some(true) {
                    "emitted".to_owned()
                } else {
                    why.clone().unwrap_or_default()
                }
            );
            if ok == Some(true) && !skipped.is_empty() {
                rec.text.push_str(&format!(
                    " ({} skipped: the step names no caller)",
                    skipped
                        .iter()
                        .map(|x| format!("`{x}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if ok.is_none() {
                rec.unsupported = true;
                rec.text = format!(
                    "event {event}: emitted; its shape is not evaluable: {}",
                    why.unwrap_or_default()
                );
            }
            rec.ok = ok;
        }
        "expect_no_event" => {
            let event = s(st, "event");
            let ok = !ctx.last.events.iter().any(|e| e.event == event);
            rec.ok = Some(ok);
            rec.text = format!("no event {event}: {}", if ok { "none" } else { "emitted" });
        }
        "expect_no_events" => {
            rec.ok = Some(ctx.last.events.is_empty());
            rec.text = format!("no events: {} emitted", ctx.last.events.len());
        }
        "capture_instance" => {
            let event = s(st, "event");
            let field = s(st, "field");
            let hit = ctx
                .last
                .events
                .iter()
                .chain(ctx.last.delivered.iter())
                .rev()
                .find(|e| e.event == event);
            let val = hit.and_then(|h| h.payload.get(field)).map(jv);
            if let Some(v) = &val {
                ctx.captures.insert(s(st, "instance").to_owned(), v.clone());
            }
            rec.ok = Some(val.is_some());
            rec.text = format!(
                "capture {} = {}",
                s(st, "instance"),
                val.map_or_else(
                    || "None".into(),
                    |v| v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned)
                )
            );
        }
        "query_view" => {
            let view = s(st, "view").to_owned();
            let params = ctx.view_params(st)?;
            ctx.view_params.insert(view.clone(), params.clone());
            let rows = ctx.rows(&view, params.as_ref())?;
            rec.ok = Some(true);
            rec.text = format!("query {view}: {} row(s)", rows.len());
            rec.view = Some(view);
        }
        "snapshot_view" => {
            let view = s(st, "view").to_owned();
            let params = ctx.view_params(st)?;
            let rows = ctx.rows(&view, params.as_ref())?;
            rec.ok = Some(true);
            rec.text = format!("snapshot {view}: {} row(s)", rows.len());
            ctx.view_snapshots.insert(view.clone(), rows);
            rec.view = Some(view);
        }
        "expect_view" | "eventually_view" => {
            let view = s(st, "view").to_owned();
            rec.view = Some(view.clone());
            if let Some(field) = open_order_key(ctx, st)? {
                // A position or a ranking over a field no outcome writes is the implementation's
                // to decide: not evaluable here, never a failure the page invented.
                rec.ok = None;
                rec.unsupported = true;
                rec.text = format!("{view}: the order reads `{field}`, which the model leaves to the implementation; not evaluable");
                return Ok(());
            }
            let (ok, why) = view_expect(ctx, st)?;
            rec.ok = Some(ok);
            rec.text = format!("{view}: {why}");
        }
        "snapshot_complete_subject" => {
            let view = s(st, "view").to_owned();
            let idv = ctx.resolve_map(st.get("subject"))?;
            let rows = ctx.rows(&view, None)?;
            let row = rows.iter().map(|r| jfields(&r.1)).find(|r| {
                idv.iter()
                    .all(|(k, v)| r.get(k).is_some_and(|g| values_equal(g, v)))
            });
            rec.ok = Some(true);
            rec.text = format!("snapshot {view} row for {}", Value::Object(idv.clone()));
            ctx.snapshots.insert(view.clone(), (idv, row));
            rec.view = Some(view);
        }
        "expect_complete_subject_unchanged" => {
            let view = s(st, "view").to_owned();
            let (idv, row) = ctx.snapshots.get(&view).cloned().unwrap_or_default();
            let rows = ctx.rows(&view, None)?;
            let now = rows.iter().map(|r| jfields(&r.1)).find(|r| {
                idv.iter()
                    .all(|(k, v)| r.get(k).is_some_and(|g| values_equal(g, v)))
            });
            let ok = match (&now, &row) {
                (Some(a), Some(b)) => values_equal(a, b),
                (a, b) => a == b,
            };
            rec.ok = Some(ok);
            rec.text = format!(
                "{view} row unchanged: {}",
                if ok { "yes" } else { "changed" }
            );
            rec.view = Some(view);
        }
        "expect_subject_absent" => {
            let view = s(st, "view").to_owned();
            let idv = ctx.resolve_map(st.get("subject"))?;
            let rows = ctx.rows(&view, None)?;
            let hit = rows.iter().map(|r| jfields(&r.1)).any(|r| {
                idv.iter()
                    .all(|(k, v)| r.get(k).is_some_and(|g| values_equal(g, v)))
            });
            rec.ok = Some(!hit && idv.values().all(|v| !v.is_null()));
            rec.text = format!(
                "{view} holds no row for {}: {}",
                Value::Object(idv),
                if hit { "a row is there" } else { "holds" }
            );
            rec.view = Some(view);
        }
        "configure_external_outcome" => {
            let force = st.get("force").cloned().unwrap_or(Value::Null);
            ctx.forced.insert(
                s(&force, "command").to_owned(),
                s(&force, "outcome").to_owned(),
            );
            rec.ok = Some(true);
            rec.text = format!("force {} → {}", s(&force, "command"), s(&force, "outcome"));
        }
        "redeliver_event" => {
            let event = s(st, "event");
            let hit = ctx
                .last
                .events
                .iter()
                .chain(ctx.last.delivered.iter())
                .rev()
                .find(|e| e.event == event)
                .cloned();
            if let Some(hit) = &hit {
                let forced = ctx.forced.clone();
                let (delivered, invocations) =
                    ctx.interp
                        .deliver(&mut ctx.engine, std::slice::from_ref(hit), &forced, 0);
                ctx.history.extend(delivered.iter().cloned());
                ctx.last.delivered.extend(delivered);
                ctx.last.invocations.extend(invocations);
            }
            rec.ok = Some(hit.is_some());
            rec.text = format!("redeliver {event}");
        }
        "expect_invocation" => {
            let want = ctx.resolve_map(st.get("input"))?;
            let binding = s(st, "binding");
            let command = s(st, "command");
            let ok = ctx.last.invocations.iter().any(|i| {
                i.binding == binding
                    && i.command == command
                    && want
                        .iter()
                        .all(|(k, v)| i.input.get(k).map(jv).is_some_and(|g| values_equal(&g, v)))
            });
            rec.ok = Some(ok);
            rec.text = format!(
                "binding {binding} invoked {command}: {}",
                if ok { "yes" } else { "no" }
            );
        }
        other => {
            rec.ok = None;
            rec.unsupported = true;
            rec.text = format!("step `{other}` is not evaluated");
        }
    }
    Ok(())
}

/// The first order key of an `at` or `ranked` expectation some row does not hold.
fn open_order_key(ctx: &Ctx<'_, '_>, st: &Value) -> Result<Option<String>, Undetermined> {
    let ex = st.get("expectation").cloned().unwrap_or(Value::Null);
    if !matches!(s(&ex, "expect"), "at" | "ranked") {
        return Ok(None);
    }
    let params = ctx.view_params(st)?;
    let rows = ctx.rows(s(st, "view"), params.as_ref())?;
    for key in ex
        .get("order_by")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let field = match key {
            Value::String(t) => t.split_whitespace().next().unwrap_or("").to_owned(),
            other => s(other, "field").to_owned(),
        };
        if rows
            .iter()
            .any(|(_, r)| crate::interp::lookup(r, &field).is_none())
        {
            return Ok(Some(field));
        }
    }
    Ok(None)
}

fn view_expect(ctx: &Ctx<'_, '_>, st: &Value) -> Result<(bool, String), Undetermined> {
    let view = s(st, "view");
    let params = ctx.view_params(st)?;
    let rows = ctx.rows(view, params.as_ref())?;
    let ex = st.get("expectation").cloned().unwrap_or(Value::Null);
    let rows_json: Vec<Value> = rows.iter().map(|r| jfields(&r.1)).collect();
    let fields_text = |m: &Map<String, Value>| {
        m.iter()
            .map(|(k, v)| format!("{k} = {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(match s(&ex, "expect") {
        "contains" => {
            let want = ctx.resolve_map(ex.get("fields"))?;
            let ok = rows_json.iter().any(|r| row_matches(r, &want));
            (
                ok,
                format!(
                    "{} row(s); a row with {} {}",
                    rows.len(),
                    fields_text(&want),
                    if ok { "found" } else { "not found" }
                ),
            )
        }
        "excludes" => {
            let want = ctx.resolve_map(ex.get("fields"))?;
            let ok = !rows_json.iter().any(|r| row_matches(r, &want));
            (
                ok,
                format!(
                    "{} row(s); no row with {}: {}",
                    rows.len(),
                    fields_text(&want),
                    if ok { "holds" } else { "violated" }
                ),
            )
        }
        "at" => {
            // Rows are already in the view's declared order, which is the order the claim names.
            let want = ctx.resolve_map(ex.get("fields"))?;
            let pos = ex.get("position").cloned().unwrap_or(Value::Null);
            let (label, row) = match (s(&pos, "row"), pos.get("index").and_then(Value::as_u64)) {
                ("first", _) => ("first".to_owned(), rows_json.first()),
                ("last", _) => ("last".to_owned(), rows_json.last()),
                (_, Some(i)) => (
                    format!("row {i}"),
                    usize::try_from(i).ok().and_then(|i| rows_json.get(i)),
                ),
                (other, None) => return Err(Undetermined(format!("view position `{other}`"))),
            };
            let ok = row.is_some_and(|r| row_matches(r, &want));
            (
                ok,
                format!(
                    "{} row(s); the {label} row has {}: {}",
                    rows.len(),
                    fields_text(&want),
                    if ok { "holds" } else { "violated" }
                ),
            )
        }
        "counts" => {
            let n = rows.len() as u64;
            let get = |k: &str| ex.get(k).and_then(Value::as_u64);
            let ok = get("at_least").is_none_or(|x| n >= x)
                && get("at_most").is_none_or(|x| n <= x)
                && get("exactly").is_none_or(|x| n == x);
            (
                ok,
                format!(
                    "{n} row(s), expected {}",
                    Value::Object(
                        ex.as_object()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|(k, _)| k != "expect")
                            .collect()
                    )
                ),
            )
        }
        "satisfies" => {
            let text = s(&ex, "predicate");
            let p = ess_primitives::predicate::Predicate::parse_expression(text)
                .map_err(|e| Undetermined(format!("the predicate `{text}`: {e}")))?;
            let v = ctx
                .interp
                .ir
                .views()
                .iter()
                .find(|(k, _)| k.to_string() == view)
                .map(|(_, v)| v.fields.clone())
                .unwrap_or_default();
            let mut bad = 0;
            for (_, row) in &rows {
                let mut f = crate::facts::Facts::new(ctx.interp.ir);
                f.bind(None, &v, row).map_err(Undetermined)?;
                if f.decide(&p) == Some(false) {
                    bad += 1;
                }
            }
            (
                bad == 0,
                format!(
                    "every row satisfies `{text}`: {}",
                    if bad == 0 {
                        "holds".to_owned()
                    } else {
                        format!("{bad} row(s) do not")
                    }
                ),
            )
        }
        "changed_by" => {
            let before = ctx.view_snapshots.get(view).ok_or_else(|| {
                Undetermined(format!("`changed_by` with no snapshot of `{view}`"))
            })?;
            let now = rows_json.first().cloned().unwrap_or(json!({}));
            let was = before.first().map_or(json!({}), |r| jfields(&r.1));
            let mut bad = Vec::new();
            for (name, want) in ex
                .get("fields")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                let a = lookup(&now, name).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = lookup(&was, name).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let want = if want.is_object() {
                    ctx.resolve(want)?
                } else {
                    want.clone()
                };
                if want.as_f64() != Some(a - b) {
                    bad.push(format!("`{name}` changed by {}, expected {want}", a - b));
                }
            }
            (
                bad.is_empty(),
                if bad.is_empty() {
                    "changed as expected".into()
                } else {
                    bad.join("; ")
                },
            )
        }
        "ranked" => {
            let order: Vec<String> = ex
                .get("order_by")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => format!("{} {}", s(other, "field"), s(other, "direction")),
                })
                .collect();
            let mut ok = true;
            for pair in rows.windows(2) {
                for key in &order {
                    let mut parts = key.split_whitespace();
                    let f = parts.next().unwrap_or("");
                    let desc = parts
                        .next()
                        .is_some_and(|d| d.to_lowercase().starts_with("desc"));
                    let (Some(a), Some(b)) = (
                        crate::interp::lookup(&pair[0].1, f),
                        crate::interp::lookup(&pair[1].1, f),
                    ) else {
                        break;
                    };
                    let (ka, kb) = (crate::interp::sort_key(a), crate::interp::sort_key(b));
                    if ka == kb {
                        continue;
                    }
                    if (ka > kb) != desc {
                        ok = false;
                    }
                    break;
                }
            }
            (
                ok,
                format!(
                    "{} row(s) ranked by {}: {}",
                    rows.len(),
                    order.join(", "),
                    if ok { "holds" } else { "violated" }
                ),
            )
        }
        other => return Err(Undetermined(format!("view expectation `{other}`"))),
    })
}

/// Replays every scenario of a suite: traces by name, and the tally per step kind
/// `[met, evaluated, not evaluable]`.
pub fn replay_suite<'a>(
    interp: &'a Interp<'a>,
    suite: &Value,
    authored: bool,
) -> (
    BTreeMap<String, Vec<Rec>>,
    BTreeMap<String, [usize; 3]>,
    Vec<String>,
) {
    let mut traces = BTreeMap::new();
    let mut tally: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    let mut divergences = Vec::new();
    for (name, sc) in suite
        .get("scenarios")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let (tr, engine) = replay(interp, sc, authored);
        divergences.extend(engine.divergences.iter().map(|d| format!("{name}: {d}")));
        for r in &tr {
            let t = tally.entry(r.step.clone()).or_insert([0; 3]);
            if r.unsupported {
                t[2] += 1;
            } else {
                t[1] += 1;
                if r.ok == Some(true) {
                    t[0] += 1;
                }
            }
        }
        traces.insert(name.clone(), tr);
    }
    (traces, tally, divergences)
}

/// Every failed step, as `<scenario> step <n> <kind>: <text>`.
pub fn failures(traces: &BTreeMap<String, Vec<Rec>>) -> Vec<String> {
    let mut out = Vec::new();
    for (name, tr) in traces {
        for (i, r) in tr.iter().enumerate() {
            if r.ok == Some(false) {
                out.push(format!("{name} step {} {}: {}", i + 1, r.step, r.text));
            }
        }
    }
    out
}
