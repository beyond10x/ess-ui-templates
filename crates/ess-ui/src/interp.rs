//! Executes commands the way the compiled model says: ESS's interpreter first, and the page's
//! extensions where ESS does not execute a construct yet.
//!
//! [`Engine::execute`] hands every command to ESS's own interpreter
//! (`ess_conformance::interpret::execute`) while ESS holds the same store as the page. Where ESS
//! answers exactly one step, that step is the answer, and the page's own reading of the same
//! command is compared with it (a difference is recorded as a divergence; the tests require none).
//! Where ESS leaves the outcome open or reports a construct it does not execute, the page's
//! interpreter answers ([`Interp::step`]). ESS's store cannot be built from outside, so after a
//! page-executed step that changes the store, ESS is no longer consulted for that run.
//!
//! The page's interpreter follows ESS's precedence and adds what ESS does not execute yet:
//!
//! - `existing_instance:`, and a creation whose identity the caller supplies (`input.f`, or
//!   `{input: f, else: {generated: true}}`);
//! - `when_related:` with a `holds` predicate over the related row and `input.`, and the input guard
//!   beside it; two related-row or accepting branches that hold leave the step open, as ESS's
//!   synthesis selects none of them;
//! - `when_subject:` (`subject_predicate`, and the ess/6 `subject_field`) over held fields with the
//!   input guard beside it;
//! - `when_subject_state:` over the held state, for a refusal naming no subject reading its
//!   siblings' subject;
//! - `sets:` from `increment`, a related row's field, the caller's attribute, a struct, and a minted
//!   `Timestamp`; a minted identity of a `String` or `Integer` type;
//! - bindings: emitted events delivered synchronously, with one retry and escalation;
//! - views: rows derived with filter, parameters, fields, aggregation and `order_by`.

use std::collections::BTreeMap;
use std::fmt;

use ess_compiler::ir::{
    EssIr, ResolvedBody, ResolvedCommand, ResolvedCondition, ResolvedEffect, ResolvedInstance,
    ResolvedMappingValue, ResolvedOutcome, ResolvedPayloadField, ResolvedPayloadValue,
    ResolvedRelatedTest, ResolvedRelatedVia, ResolvedSubject, ResolvedTypeRef, ResolvedView,
};
use ess_conformance::interpret::execute as ess_exec;
use ess_domain::binding::Failure;
use ess_domain::name::QualifiedName;
use ess_domain::types::Primitive;
use ess_domain::view::{AggregateFunction, Direction};
use ess_primitives::facts::Number;
use ess_primitives::node::Node;
use ess_primitives::predicate::Predicate;

use crate::facts::Facts;

/// A field map, as ESS holds one.
pub type Fields = BTreeMap<String, Node>;

/// Why the model, as the page reads it, determines no answer. Never a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undetermined(pub String);

impl fmt::Display for Undetermined {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn undetermined<T>(why: impl Into<String>) -> Result<T, Undetermined> {
    Err(Undetermined(why.into()))
}

/// One held instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inst {
    /// Its identity, as the caller or the minting counter gave it.
    pub id: Node,
    /// Where its lifecycle rests.
    pub state: String,
    /// What outcomes have written.
    pub fields: Fields,
}

/// Every held instance, by entity and identity text, and the minting counter.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct World {
    /// Entity → identity text → instance.
    pub inst: BTreeMap<String, BTreeMap<String, Inst>>,
    /// The counter every minted value comes from.
    pub minted: u64,
}

impl World {
    fn get(&self, entity: &str, key: &str) -> Option<&Inst> {
        self.inst.get(entity)?.get(key)
    }

    /// Whether two worlds hold the same instances (the counter aside).
    pub fn same_rows(&self, other: &Self) -> bool {
        let strip = |w: &Self| -> BTreeMap<String, BTreeMap<String, (String, Fields)>> {
            w.inst
                .iter()
                .filter(|(_, held)| !held.is_empty())
                .map(|(e, held)| {
                    (
                        e.clone(),
                        held.iter()
                            .map(|(k, i)| (k.clone(), (i.state.clone(), i.fields.clone())))
                            .collect(),
                    )
                })
                .collect()
        };
        strip(self) == strip(other)
    }
}

/// The text an identity is keyed by.
pub fn key_of(id: &Node) -> String {
    match id {
        Node::Text(t) => t.clone(),
        Node::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

/// One emitted event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emitted {
    /// The event's qualified name.
    pub event: String,
    /// Its payload.
    pub payload: Fields,
    /// Fields the model leaves to the implementation, so they stay absent.
    pub open: Vec<String>,
    /// Fields read from the caller where the step names no caller.
    pub uncalled: Vec<String>,
}

/// Which interpreter answered a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// ESS's interpreter.
    Ess,
    /// The page's extension of it.
    Page,
}

/// One executed command.
#[derive(Debug, Clone)]
pub struct Step {
    /// The declared branch taken; `None` for a refusal the model does not declare.
    pub outcome: Option<String>,
    /// The declared error it reports.
    pub error: Option<String>,
    /// The events it emits.
    pub events: Vec<Emitted>,
    /// The world after it.
    pub world: World,
    /// Whether it refused.
    pub refused: bool,
    /// What the page says about it.
    pub note: Option<String>,
    /// An invariant the outcome leaves broken.
    pub broken: Option<String>,
    /// The answers the model leaves open, first declared first.
    pub open: Vec<(Option<String>, Option<String>)>,
    /// Who answered.
    pub by: By,
}

/// A binding's invocation.
#[derive(Debug, Clone)]
pub struct Invocation {
    /// The binding.
    pub binding: String,
    /// The command it sent.
    pub command: String,
    /// The input it sent.
    pub input: Fields,
    /// What answered.
    pub outcome: Option<String>,
    /// The error.
    pub error: Option<String>,
    /// Whether this is the retry.
    pub retry: bool,
    /// Why it was not executed.
    pub note: Option<String>,
}

/// The page's interpreter over one compiled model.
pub struct Interp<'ir> {
    /// The model.
    pub ir: &'ir EssIr,
    commands: BTreeMap<String, &'ir ResolvedCommand>,
}

fn is_default(o: &ResolvedOutcome) -> bool {
    match &o.condition {
        ResolvedCondition::Otherwise => true,
        ResolvedCondition::When { predicate } => predicate.is_trivially_true(),
        _ => false,
    }
}

fn is_external(o: &ResolvedOutcome) -> bool {
    matches!(
        o.condition,
        ResolvedCondition::External { .. } | ResolvedCondition::ExternalWhen { .. }
    )
}

enum Rep {
    Prim(Primitive),
    Enum,
    Other,
}

impl<'ir> Interp<'ir> {
    /// The interpreter of `ir`.
    pub fn new(ir: &'ir EssIr) -> Self {
        Self {
            ir,
            commands: ir
                .commands()
                .iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    /// The command named `name`.
    pub fn command(&self, name: &str) -> Option<&'ir ResolvedCommand> {
        self.commands.get(name).copied()
    }

    fn rep(&self, t: &ResolvedTypeRef) -> Rep {
        let mut cur = t.required();
        for _ in 0..=ess_domain::types::MAX_TYPE_DEPTH {
            match cur {
                ResolvedTypeRef::Primitive { name } => return Rep::Prim(*name),
                ResolvedTypeRef::Declared { name } => match &self.ir.named_type(name).body {
                    ResolvedBody::Newtype { of, .. } => cur = of.required(),
                    ResolvedBody::Enum { .. } => return Rep::Enum,
                    _ => return Rep::Other,
                },
                _ => return Rep::Other,
            }
        }
        Rep::Other
    }

    /// The variants of an enum type, through newtypes and `Optional`.
    pub fn variants(&self, t: &ResolvedTypeRef) -> Vec<String> {
        let mut cur = t.required();
        for _ in 0..=ess_domain::types::MAX_TYPE_DEPTH {
            match cur {
                ResolvedTypeRef::Declared { name } => match &self.ir.named_type(name).body {
                    ResolvedBody::Newtype { of, .. } => cur = of.required(),
                    ResolvedBody::Enum { variants } => {
                        return variants.iter().map(|v| v.name.clone()).collect()
                    }
                    _ => return Vec::new(),
                },
                _ => return Vec::new(),
            }
        }
        Vec::new()
    }

    /// The typed value a literal written in the model denotes.
    pub fn literal(&self, t: &ResolvedTypeRef, text: &str) -> Result<Node, Undetermined> {
        let gap = || Undetermined(format!("the literal `{text}` over `{t}`"));
        match self.rep(t) {
            Rep::Prim(Primitive::Integer) => text
                .parse::<i64>()
                .map(|v| Node::Number(Number::from(v)))
                .map_err(|_| gap()),
            Rep::Prim(Primitive::Decimal | Primitive::Binary64) => Number::decimal_literal(text)
                .map(Node::Number)
                .ok_or_else(gap),
            Rep::Prim(Primitive::Boolean) => match text {
                "true" => Ok(Node::Bool(true)),
                "false" => Ok(Node::Bool(false)),
                _ => Err(gap()),
            },
            _ => Ok(Node::Text(text.to_owned())),
        }
    }

    fn mint(&self, t: &ResolvedTypeRef, w: &mut World) -> Option<Node> {
        if t.is_optional() {
            return None;
        }
        match self.rep(t) {
            Rep::Prim(Primitive::Uuid) => {
                w.minted += 1;
                Some(Node::Text(format!(
                    "00000000-0000-4000-8000-{:012}",
                    w.minted
                )))
            }
            // An instant the implementation stamps: the model holds the field to carrying one, not
            // to its value, so any instant stands for it.
            Rep::Prim(Primitive::Timestamp) => {
                w.minted += 1;
                Some(Node::Text(format!("2000-01-01T00:00:00.{:06}Z", w.minted)))
            }
            _ => None,
        }
    }

    /// A new instance's identity the model leaves to the implementation: a Uuid as ESS mints it,
    /// and a String (the same text) or an Integer (the counter) where ESS mints Uuids only. A value
    /// already held is skipped; a type whose newtypes carry invariants is not minted.
    fn mint_identity(&self, t: &ResolvedTypeRef, w: &mut World, held: &[String]) -> Option<Node> {
        if t.is_optional() {
            return None;
        }
        let mut cur = t;
        for _ in 0..=ess_domain::types::MAX_TYPE_DEPTH {
            match cur {
                ResolvedTypeRef::Declared { name } => match &self.ir.named_type(name).body {
                    body @ ResolvedBody::Newtype { of, .. } => {
                        if body.is_constrained() {
                            return None;
                        }
                        cur = of;
                    }
                    _ => break,
                },
                _ => break,
            }
        }
        let name = match self.rep(t) {
            Rep::Prim(p @ (Primitive::Uuid | Primitive::String | Primitive::Integer)) => p,
            _ => return None,
        };
        loop {
            w.minted += 1;
            let v = if name == Primitive::Integer {
                Node::Number(Number::from(i64::try_from(w.minted).unwrap_or(i64::MAX)))
            } else {
                Node::Text(format!("00000000-0000-4000-8000-{:012}", w.minted))
            };
            if !held.contains(&key_of(&v)) {
                return Some(v);
            }
        }
    }

    fn input_facts(
        &self,
        cmd: &ResolvedCommand,
        input: &Fields,
    ) -> Result<Facts<'ir>, Undetermined> {
        let mut f = Facts::new(self.ir);
        f.bind(None, &cmd.input, input)
            .map_err(|e| Undetermined(format!("the input of `{}`: {e}", cmd.name)))?;
        Ok(f)
    }

    fn holds_input(
        &self,
        cmd: &ResolvedCommand,
        p: &Predicate,
        input: &Fields,
    ) -> Result<bool, Undetermined> {
        Ok(self.input_facts(cmd, input)?.decide(p) == Some(true))
    }

    /// The facts of a held row: its fields, identity and `state`, and the input as `input.`.
    fn row_facts(
        &self,
        entity: &QualifiedName,
        inst: &Inst,
        input: Option<(&ResolvedCommand, &Fields)>,
    ) -> Result<Facts<'ir>, Undetermined> {
        let e = &self.ir.entities()[entity];
        let mut f = Facts::new(self.ir);
        let mut fields = e.fields.clone();
        fields.push(e.identity.clone());
        let mut values = inst.fields.clone();
        values.insert(e.identity.name.clone(), inst.id.clone());
        f.bind(None, &fields, &values)
            .map_err(|why| Undetermined(format!("the held `{entity}`: {why}")))?;
        f.text("state", &inst.state);
        if let Some((cmd, input)) = input {
            f.bind(Some("input"), &cmd.input, input)
                .map_err(|why| Undetermined(format!("the input of `{}`: {why}", cmd.name)))?;
        }
        Ok(f)
    }

    fn entity_name(&self, s: &ResolvedSubject) -> &'ir QualifiedName {
        &self.ir.entity(&s.entity).name
    }

    fn refusal(o: &ResolvedOutcome, w: &World) -> Step {
        Step {
            outcome: Some(o.name.to_string()),
            error: o.error.as_ref().map(ToString::to_string),
            events: Vec::new(),
            world: w.clone(),
            refused: true,
            note: None,
            broken: None,
            open: Vec::new(),
            by: By::Page,
        }
    }

    fn undeclared(w: &World, note: impl Into<String>) -> Step {
        Step {
            outcome: None,
            error: None,
            events: Vec::new(),
            world: w.clone(),
            refused: true,
            note: Some(note.into()),
            broken: None,
            open: Vec::new(),
            by: By::Page,
        }
    }

    fn find(
        cmd: &ResolvedCommand,
        f: impl Fn(&ResolvedCondition) -> bool,
    ) -> Option<&ResolvedOutcome> {
        cmd.outcomes.iter().find(|o| f(&o.condition))
    }

    /// The subject a `when_subject_state:` or `when_subject:` branch reads.
    fn state_subject<'c>(
        cmd: &'c ResolvedCommand,
        o: &'c ResolvedOutcome,
    ) -> Option<&'c ResolvedSubject> {
        o.subject.as_ref().or_else(|| {
            cmd.outcomes
                .iter()
                .filter_map(|x| x.subject.as_ref())
                .find(|s| matches!(s.instance, ResolvedInstance::Supplied { .. }))
        })
    }

    /// The identity a `creates:` takes from the caller's input; `None` when it is minted.
    fn supplied_identity(o: &ResolvedOutcome, input: &Fields) -> Option<Option<Node>> {
        let s = o.subject.as_ref()?;
        let ResolvedInstance::Observed { event, field } = &s.instance else {
            return None;
        };
        let p = o.payload.iter().find(|p| &p.event == event)?;
        let src = p.fields.iter().find(|f| f.target == field.name)?;
        match &src.value {
            ResolvedPayloadValue::InputField { field: f, .. } => {
                Some(input.get(f).filter(|v| **v != Node::Null).cloned())
            }
            ResolvedPayloadValue::InputOrGenerated { field: f, .. } => {
                let got = input.get(f).filter(|v| **v != Node::Null).cloned();
                got.map(Some)
            }
            _ => None,
        }
    }

    fn existing_instance<'c>(
        &self,
        cmd: &'c ResolvedCommand,
        w: &World,
        input: &Fields,
    ) -> Option<&'c ResolvedOutcome> {
        let taken = Self::find(cmd, |c| matches!(c, ResolvedCondition::ExistingInstance))?;
        for o in &cmd.outcomes {
            let Some(s) = &o.subject else { continue };
            if !matches!(s.effect, ResolvedEffect::Creates) {
                continue;
            }
            if let Some(Some(id)) = Self::supplied_identity(o, input) {
                if w.get(&self.entity_name(s).to_string(), &key_of(&id))
                    .is_some()
                {
                    return Some(taken);
                }
            }
        }
        None
    }

    /// The `exists: false` branch where the related row is missing; else the row's facts.
    #[allow(clippy::type_complexity)]
    fn related_row<'c>(
        &self,
        cmd: &'c ResolvedCommand,
        w: &World,
        input: &Fields,
    ) -> Result<(Option<&'c ResolvedOutcome>, Option<Facts<'ir>>), Undetermined> {
        let rel: Vec<&ResolvedOutcome> = cmd
            .outcomes
            .iter()
            .filter(|o| matches!(o.condition, ResolvedCondition::Related { .. }))
            .collect();
        let Some(first) = rel.first() else {
            return Ok((None, None));
        };
        let ResolvedCondition::Related { via, entity, .. } = &first.condition else {
            unreachable!()
        };
        for o in &rel[1..] {
            if let ResolvedCondition::Related {
                via: v2,
                entity: e2,
                ..
            } = &o.condition
            {
                if v2 != via || e2 != entity {
                    return undetermined("related guards over more than one row");
                }
            }
        }
        let ResolvedRelatedVia::Input { field, .. } = via else {
            return undetermined(format!("a related row read through `{via}`"));
        };
        let ename = &self.ir.entity(entity).name;
        let ident = input.get(field).map(key_of);
        let held = ident.as_deref().and_then(|k| w.get(&ename.to_string(), k));
        let Some(held) = held else {
            let absent = rel.iter().find(|o| {
                matches!(
                    &o.condition,
                    ResolvedCondition::Related {
                        test: ResolvedRelatedTest::Absent,
                        ..
                    }
                )
            });
            return match absent {
                Some(a) => Ok((Some(a), None)),
                None => undetermined(format!(
                    "no `{ename}` `{}`, and no `exists: false` branch declared",
                    ident.unwrap_or_default()
                )),
            };
        };
        Ok((None, Some(self.row_facts(ename, held, Some((cmd, input)))?)))
    }

    fn input_refusal<'c>(
        &self,
        cmd: &'c ResolvedCommand,
        input: &Fields,
    ) -> Result<Option<&'c ResolvedOutcome>, Undetermined> {
        for o in &cmd.outcomes {
            if let ResolvedCondition::When { predicate } = &o.condition {
                if o.error.is_some()
                    && o.subject.is_none()
                    && !is_default(o)
                    && self.holds_input(cmd, predicate, input)?
                {
                    return Ok(Some(o));
                }
            }
        }
        Ok(None)
    }

    fn related_holding<'c>(
        &self,
        cmd: &'c ResolvedCommand,
        input: &Fields,
        row: Option<&Facts<'_>>,
    ) -> Result<Vec<&'c ResolvedOutcome>, Undetermined> {
        let mut sel = Vec::new();
        let Some(row) = row else { return Ok(sel) };
        for o in &cmd.outcomes {
            let ResolvedCondition::Related {
                test, input: guard, ..
            } = &o.condition
            else {
                continue;
            };
            let ResolvedRelatedTest::Holds { predicate } = test else {
                continue;
            };
            if row.decide(predicate) != Some(true) {
                continue;
            }
            if let Some(g) = guard {
                if !self.holds_input(cmd, g, input)? {
                    continue;
                }
            }
            sel.push(o);
        }
        Ok(sel)
    }

    fn accepting<'c>(
        &self,
        cmd: &'c ResolvedCommand,
        input: &Fields,
        related: Vec<&'c ResolvedOutcome>,
    ) -> Result<Vec<&'c ResolvedOutcome>, Undetermined> {
        let mut holding: Vec<&ResolvedOutcome> = Vec::new();
        for o in &cmd.outcomes {
            if let ResolvedCondition::When { predicate } = &o.condition {
                if !is_default(o) && self.holds_input(cmd, predicate, input)? {
                    holding.push(o);
                }
            }
        }
        if holding.is_empty() {
            return Ok(related);
        }
        let has_related = cmd
            .outcomes
            .iter()
            .any(|o| matches!(o.condition, ResolvedCondition::Related { .. }));
        if !has_related {
            holding.truncate(1);
        }
        Ok(cmd
            .outcomes
            .iter()
            .filter(|o| {
                holding.iter().any(|h| std::ptr::eq(*h, *o))
                    || related.iter().any(|h| std::ptr::eq(*h, *o))
            })
            .collect())
    }

    /// The step the model determines for `command` with `input` against `w`.
    pub fn step(
        &self,
        w: &World,
        command: &str,
        input: &Fields,
        forced: Option<&str>,
        caller: Option<&Fields>,
    ) -> Result<Step, Undetermined> {
        let Some(cmd) = self.command(command) else {
            return undetermined(format!("no command `{command}`"));
        };
        for o in &cmd.outcomes {
            if o.instances.is_some() || !o.affects.is_empty() {
                return undetermined(format!("the set effect of `{command}/{}`", o.name));
            }
            if matches!(
                o.condition,
                ResolvedCondition::StateChange { .. } | ResolvedCondition::InputAbsent
            ) {
                return undetermined(format!("the condition of `{command}/{}`", o.name));
            }
        }
        let outs = &cmd.outcomes;
        let related = outs
            .iter()
            .any(|o| matches!(o.condition, ResolvedCondition::Related { .. }));
        if related {
            if let Some(t) = self.existing_instance(cmd, w, input) {
                return Ok(Self::refusal(t, w));
            }
        }
        let (absent, row) = self.related_row(cmd, w, input)?;
        if let Some(a) = absent {
            return if a.error.is_some() {
                Ok(Self::refusal(a, w))
            } else {
                self.take(cmd, a, w, input, caller, false)
            };
        }
        if let Some(r) = self.input_refusal(cmd, input)? {
            return Ok(Self::refusal(r, w));
        }
        if !related {
            if let Some(t) = self.existing_instance(cmd, w, input) {
                return Ok(Self::refusal(t, w));
            }
        }
        let sel = self.related_holding(cmd, input, row.as_ref())?;
        let mut sel = self.accepting(cmd, input, sel)?;
        if sel.is_empty() {
            let forced_external = forced.is_some_and(|f| {
                outs.iter()
                    .any(|o| o.name.to_string() == f && is_external(o))
            });
            let unknown = if forced_external {
                None
            } else {
                Self::find(cmd, |c| matches!(c, ResolvedCondition::UnknownInstance))
            };
            let state = Self::find(cmd, |c| matches!(c, ResolvedCondition::SubjectState { .. }));
            if let (Some(unknown), Some(state)) = (unknown, state) {
                if let Some(s) = Self::state_subject(cmd, state) {
                    let id = input.get(&s.instance.field().name).map(key_of);
                    if id
                        .as_deref()
                        .and_then(|k| w.get(&self.entity_name(s).to_string(), k))
                        .is_none()
                    {
                        return Ok(Self::refusal(unknown, w));
                    }
                }
            }
            for o in outs {
                let ResolvedCondition::SubjectState { state, predicate } = &o.condition else {
                    continue;
                };
                let Some(s) = Self::state_subject(cmd, o) else {
                    continue;
                };
                let id = input.get(&s.instance.field().name).map(key_of);
                let Some(held) = id
                    .as_deref()
                    .and_then(|k| w.get(&self.entity_name(s).to_string(), k))
                else {
                    continue;
                };
                if !state.iter().any(|x| x.as_str() == held.state) {
                    continue;
                }
                if let Some(p) = predicate {
                    if !self.holds_input(cmd, p, input)? {
                        continue;
                    }
                }
                sel.push(o);
            }
        }
        if sel.is_empty() {
            sel = outs.iter().filter(|o| is_default(o)).collect();
        }
        if let Some(f) = forced {
            if let Some(fo) = outs
                .iter()
                .find(|o| o.name.to_string() == f && is_external(o))
            {
                sel = vec![fo];
            }
        }
        if sel.is_empty() {
            return Ok(Self::undeclared(w, "no declared branch covers the request"));
        }
        let mut steps: Vec<Step> = Vec::new();
        for o in sel {
            let s = self.take(cmd, o, w, input, caller, false)?;
            if !steps
                .iter()
                .any(|x| x.outcome == s.outcome && x.world == s.world)
            {
                steps.push(s);
            }
        }
        let mut first = steps.remove(0);
        if !steps.is_empty() {
            let mut all = vec![(first.outcome.clone(), first.error.clone())];
            all.extend(steps.iter().map(|s| (s.outcome.clone(), s.error.clone())));
            first.note = Some(format!(
                "the model leaves {} open; the first is shown",
                all.iter()
                    .map(|(o, _)| o.clone().unwrap_or_else(|| "none".into()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            first.open = all;
        }
        Ok(first)
    }

    #[allow(clippy::too_many_arguments)]
    fn value(
        &self,
        src: &ResolvedPayloadValue,
        t: &ResolvedTypeRef,
        input: &Fields,
        before: Option<&Fields>,
        caller: Option<&Fields>,
        work: &mut World,
        fields: Option<&Fields>,
        sets: &[ResolvedPayloadField],
    ) -> Result<Val, Undetermined> {
        Ok(match src {
            ResolvedPayloadValue::Struct { fields: members } => {
                let mut m = BTreeMap::new();
                for f in members {
                    if let Val::Is(Some(v)) = self.value(
                        &f.value,
                        &f.target_type,
                        input,
                        before,
                        caller,
                        work,
                        fields,
                        sets,
                    )? {
                        m.insert(f.target.clone(), v);
                    }
                }
                Val::Is(Some(Node::Map(m)))
            }
            ResolvedPayloadValue::CallerAttribute { attribute, .. } => {
                Val::Is(caller.and_then(|c| c.get(attribute)).cloned())
            }
            ResolvedPayloadValue::RelatedField {
                via, entity, field, ..
            } => {
                let name = via.field();
                let key = match via {
                    ResolvedRelatedVia::Input { .. } => input.get(name).cloned(),
                    ResolvedRelatedVia::Subject { .. } => before
                        .and_then(|b| b.get(name))
                        .or_else(|| fields.and_then(|f| f.get(name)))
                        .cloned()
                        .or_else(|| {
                            sets.iter()
                                .find(|s| s.target == name)
                                .and_then(|s| match &s.value {
                                    ResolvedPayloadValue::InputField { field, .. } => {
                                        input.get(field).cloned()
                                    }
                                    _ => None,
                                })
                        }),
                };
                let ename = &self.ir.entity(entity).name;
                let Some(key) = key else {
                    return Ok(Val::Is(None));
                };
                let Some(row) = work.get(&ename.to_string(), &key_of(&key)) else {
                    // A missing referenced row is the implementation's answer; the value stays open.
                    return Ok(Val::Is(None));
                };
                let e = &self.ir.entities()[ename];
                if *field == e.identity.name {
                    Val::Is(Some(key))
                } else if field == "state" {
                    Val::Is(Some(Node::Text(row.state.clone())))
                } else {
                    Val::Is(row.fields.get(field).cloned())
                }
            }
            ResolvedPayloadValue::InputField { field, .. } => {
                Val::Is(input.get(field).filter(|v| **v != Node::Null).cloned())
            }
            ResolvedPayloadValue::Literal { value } => Val::Is(Some(self.literal(t, value)?)),
            ResolvedPayloadValue::Generated => Val::Is(self.mint(t, work)),
            ResolvedPayloadValue::InputOrGenerated {
                field, otherwise, ..
            } => match input.get(field).filter(|v| **v != Node::Null) {
                Some(v) => Val::Is(Some(v.clone())),
                None => match otherwise {
                    Some(lit) => Val::Is(Some(self.literal(t, lit)?)),
                    None => Val::Is(self.mint(t, work)),
                },
            },
            ResolvedPayloadValue::Increment { by } => Val::Increment(by.clone()),
            ResolvedPayloadValue::SubjectField { field, .. } => {
                Val::Is(before.and_then(|b| b.get(field)).cloned())
            }
            ResolvedPayloadValue::Cleared => Val::Is(None),
            other => return undetermined(format!("the value source `{}`", other.describe())),
        })
    }

    fn write(
        &self,
        sets: &[ResolvedPayloadField],
        input: &Fields,
        before: Option<&Fields>,
        fields: &mut Fields,
        caller: Option<&Fields>,
        work: &mut World,
    ) -> Result<(), Undetermined> {
        for s in sets {
            let snapshot = fields.clone();
            let v = self.value(
                &s.value,
                &s.target_type,
                input,
                before,
                caller,
                work,
                Some(&snapshot),
                sets,
            )?;
            let v = match v {
                Val::Increment(by) => {
                    let old = before.and_then(|b| b.get(&s.target));
                    let Some(Node::Number(old)) = old else {
                        return undetermined(format!(
                            "`increment` of `{}`, which holds no number",
                            s.target
                        ));
                    };
                    let Node::Number(by) = self.literal(&s.target_type, &by)? else {
                        return undetermined(format!(
                            "`increment` of `{}` by a value that is not a number",
                            s.target
                        ));
                    };
                    Some(Node::Number(old.checked_add(by).ok_or_else(|| {
                        Undetermined(format!("`increment` of `{}` overflows", s.target))
                    })?))
                }
                Val::Is(v) => v,
            };
            match v {
                Some(v) => {
                    fields.insert(s.target.clone(), v);
                }
                None => {
                    fields.remove(&s.target);
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn emit(
        &self,
        o: &ResolvedOutcome,
        input: &Fields,
        created: Option<&Node>,
        before: Option<&Fields>,
        caller: Option<&Fields>,
        work: &mut World,
    ) -> Result<Vec<Emitted>, Undetermined> {
        let mut out = Vec::new();
        for handle in &o.emits {
            let decl = self.ir.event(handle);
            let determined = o.payload.iter().find(|p| &p.event == handle);
            let idfield = o.subject.as_ref().and_then(|s| match &s.instance {
                ResolvedInstance::Observed { event, field } if event == handle => {
                    Some(field.name.clone())
                }
                _ => None,
            });
            let mut payload = Fields::new();
            let mut open = Vec::new();
            let mut uncalled = Vec::new();
            for f in &decl.fields {
                let src = determined.and_then(|p| p.fields.iter().find(|x| x.target == f.name));
                if let Some(src) = src {
                    if matches!(src.value, ResolvedPayloadValue::CallerAttribute { .. })
                        && caller.is_none()
                    {
                        uncalled.push(f.name.clone());
                    }
                }
                let v = if idfield.as_deref() == Some(f.name.as_str()) && created.is_some() {
                    created.cloned()
                } else if let Some(src) = src {
                    match self.value(
                        &src.value,
                        &f.type_ref,
                        input,
                        before,
                        caller,
                        work,
                        None,
                        &o.sets,
                    )? {
                        Val::Is(v) => v,
                        Val::Increment(_) => None,
                    }
                } else {
                    self.mint(&f.type_ref, work)
                };
                match v {
                    Some(v) => {
                        payload.insert(f.name.clone(), v);
                    }
                    None => open.push(f.name.clone()),
                }
            }
            uncalled.retain(|x| open.contains(x));
            out.push(Emitted {
                event: decl.name.to_string(),
                payload,
                open,
                uncalled,
            });
        }
        Ok(out)
    }

    fn broken(
        &self,
        entity: &QualifiedName,
        w: &World,
        key: &str,
    ) -> Result<Option<String>, Undetermined> {
        let Some(inst) = w.get(&entity.to_string(), key) else {
            return Ok(None);
        };
        let facts = self.row_facts(entity, inst, None)?;
        for inv in &self.ir.entities()[entity].invariants {
            if facts.decide(&inv.predicate) == Some(false) {
                return Ok(Some(inv.statement.clone()));
            }
        }
        Ok(None)
    }

    #[allow(clippy::too_many_lines)]
    fn take(
        &self,
        cmd: &ResolvedCommand,
        o: &ResolvedOutcome,
        w: &World,
        input: &Fields,
        caller: Option<&Fields>,
        guarded: bool,
    ) -> Result<Step, Undetermined> {
        let mut work = w.clone();
        let mut created: Option<Node> = None;
        let mut touched: Vec<(QualifiedName, String)> = Vec::new();
        let mut before: Option<Fields> = None;
        if let Some(s) = &o.subject {
            let ename = self.entity_name(s).clone();
            match (&s.effect, &s.instance) {
                (ResolvedEffect::Creates, ResolvedInstance::Observed { field, .. }) => {
                    let held: Vec<String> = work
                        .inst
                        .get(&ename.to_string())
                        .map(|h| h.keys().cloned().collect())
                        .unwrap_or_default();
                    let id = match Self::supplied_identity(o, input) {
                        Some(Some(id)) => Some(id),
                        Some(None) => None,
                        None => self.mint_identity(&field.type_ref, &mut work, &held),
                    };
                    let Some(id) = id else {
                        return undetermined(format!("the identity of a new `{ename}`"));
                    };
                    let key = key_of(&id);
                    if held.contains(&key) {
                        return undetermined(format!("the identity `{key}` is already held"));
                    }
                    let mut fields = Fields::new();
                    self.write(&o.sets, input, None, &mut fields, caller, &mut work)?;
                    let state = s.into.as_ref().map_or_else(
                        || self.ir.entities()[&ename].lifecycle.initial.to_string(),
                        ToString::to_string,
                    );
                    work.inst.entry(ename.to_string()).or_default().insert(
                        key.clone(),
                        Inst {
                            id: id.clone(),
                            state,
                            fields,
                        },
                    );
                    touched.push((ename.clone(), key));
                    created = Some(id);
                }
                (ResolvedEffect::Creates, ResolvedInstance::Supplied { .. }) => {
                    return undetermined(format!(
                        "a `creates` whose identity is supplied, in `{}/{}`",
                        cmd.name, o.name
                    ));
                }
                (eff, inst) => {
                    let fname = &inst.field().name;
                    let id = input.get(fname).filter(|v| **v != Node::Null).map(key_of);
                    let held = id
                        .as_deref()
                        .and_then(|k| w.get(&ename.to_string(), k))
                        .cloned();
                    let Some(held) = held else {
                        let r =
                            Self::find(cmd, |c| matches!(c, ResolvedCondition::UnknownInstance))
                                .or_else(|| {
                                    Self::find(cmd, |c| matches!(c, ResolvedCondition::WrongState))
                                });
                        return Ok(match r {
                            Some(r) => Self::refusal(r, w),
                            None => Self::undeclared(
                                w,
                                format!(
                                    "no `{ename}` `{}`, and no refusal declared",
                                    id.unwrap_or_default()
                                ),
                            ),
                        });
                    };
                    let key = id.unwrap_or_default();
                    if !guarded {
                        let facts = self.row_facts(&ename, &held, Some((cmd, input)))?;
                        for g in &cmd.outcomes {
                            let (sub, guard) = match &g.condition {
                                ResolvedCondition::SubjectPredicate {
                                    predicate,
                                    input: guard,
                                } => (facts.decide(predicate), guard.as_ref()),
                                ResolvedCondition::SubjectField {
                                    field,
                                    equals,
                                    predicate,
                                } => (
                                    Some(
                                        matches!(held.fields.get(field), Some(Node::Text(v)) if v == equals),
                                    ),
                                    predicate.as_ref(),
                                ),
                                _ => continue,
                            };
                            let Some(sub) = sub else {
                                return undetermined(format!(
                                    "the subject guard of `{}` over a field nobody wrote",
                                    g.name
                                ));
                            };
                            let guard_holds = match guard {
                                None => true,
                                Some(p) => self.holds_input(cmd, p, input)?,
                            };
                            if sub && guard_holds {
                                if g.error.is_some() {
                                    return Ok(Self::refusal(g, w));
                                }
                                return self.take(cmd, g, w, input, caller, true);
                            }
                        }
                    }
                    before = Some(held.fields.clone());
                    let mut after = held.clone();
                    match eff {
                        ResolvedEffect::Moves { transition } => {
                            if !transition.from.iter().any(|f| f.as_str() == held.state) {
                                let r =
                                    Self::find(cmd, |c| matches!(c, ResolvedCondition::WrongState));
                                return Ok(match r {
                                    Some(r) => Self::refusal(r, w),
                                    None => Self::undeclared(
                                        w,
                                        format!(
                                            "`{}` is not a state `{}` starts from",
                                            held.state, transition.name
                                        ),
                                    ),
                                });
                            }
                            after.state = transition.to.to_string();
                        }
                        ResolvedEffect::Deletes => {
                            if let Some(h) = work.inst.get_mut(&ename.to_string()) {
                                h.remove(&key);
                            }
                        }
                        ResolvedEffect::Updates
                        | ResolvedEffect::Preserves
                        | ResolvedEffect::Creates => {}
                    }
                    if !matches!(eff, ResolvedEffect::Deletes) {
                        let b = before.clone();
                        self.write(
                            &o.sets,
                            input,
                            b.as_ref(),
                            &mut after.fields,
                            caller,
                            &mut work,
                        )?;
                        work.inst
                            .entry(ename.to_string())
                            .or_default()
                            .insert(key.clone(), after);
                        touched.push((ename.clone(), key));
                    }
                }
            }
        } else if !o.sets.is_empty() {
            return undetermined("`sets` without a subject");
        }
        let events = self.emit(
            o,
            input,
            created.as_ref(),
            before.as_ref(),
            caller,
            &mut work,
        )?;
        let mut broken = None;
        for (e, k) in &touched {
            if broken.is_none() {
                broken = self.broken(e, &work, k)?;
            }
        }
        Ok(Step {
            outcome: Some(o.name.to_string()),
            error: o.error.as_ref().map(ToString::to_string),
            events,
            world: work,
            refused: o.error.is_some() || o.subject.is_none(),
            note: None,
            broken,
            open: Vec::new(),
            by: By::Page,
        })
    }

    /// The fields of the error an outcome reports (ess/19), read against the store before the step.
    pub fn error_fields(
        &self,
        w: &World,
        command: &str,
        outcome: &str,
        input: &Fields,
        caller: Option<&Fields>,
    ) -> Fields {
        let mut out = Fields::new();
        let Some(cmd) = self.command(command) else {
            return out;
        };
        let Some(o) = cmd.outcomes.iter().find(|o| o.name.to_string() == outcome) else {
            return out;
        };
        let before = Self::state_subject(cmd, o).and_then(|s| {
            let id = input.get(&s.instance.field().name).map(key_of)?;
            w.get(&self.entity_name(s).to_string(), &id)
                .map(|i| i.fields.clone())
        });
        let mut work = w.clone();
        for f in &o.error_payload {
            if let Ok(Val::Is(Some(v))) = self.value(
                &f.value,
                &f.target_type,
                input,
                before.as_ref(),
                caller,
                &mut work,
                None,
                &[],
            ) {
                out.insert(f.target.clone(), v);
            }
        }
        out
    }

    /// The rows of a view in `w`: `(identity or group key, row)`, ordered as the view declares.
    pub fn view_rows(
        &self,
        w: &World,
        view: &str,
        params: Option<&Fields>,
    ) -> Result<Vec<(String, Fields)>, Undetermined> {
        let Some((_, v)) = self.ir.views().iter().find(|(k, _)| k.to_string() == view) else {
            return undetermined(format!("no view `{view}`"));
        };
        self.rows(w, v, params)
    }

    fn rows(
        &self,
        w: &World,
        v: &ResolvedView,
        params: Option<&Fields>,
    ) -> Result<Vec<(String, Fields)>, Undetermined> {
        let ename = &self.ir.entity(&v.source).name;
        let e = &self.ir.entities()[ename];
        let bound: Fields = params
            .map(|p| {
                p.iter()
                    .filter(|(_, x)| **x != Node::Null)
                    .map(|(k, x)| (k.clone(), x.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let filter = match &v.filter {
            None => None,
            Some(f) if v.params.is_empty() => Some(f.clone()),
            Some(f) => bind_params(f, &bound)?,
        };
        let mut rows: Vec<(String, Fields)> = Vec::new();
        if let Some(held) = w.inst.get(&ename.to_string()) {
            for (key, inst) in held {
                let mut all = inst.fields.clone();
                all.insert(e.identity.name.clone(), inst.id.clone());
                all.insert("state".into(), Node::Text(inst.state.clone()));
                if let Some(f) = &filter {
                    let mut facts = self.row_facts(ename, inst, None)?;
                    facts
                        .bind(Some("param"), &v.params, &bound)
                        .map_err(|why| {
                            Undetermined(format!("the parameters of `{}`: {why}", v.name))
                        })?;
                    if facts.decide(f) != Some(true) {
                        continue;
                    }
                }
                if v.aggregation.is_some() {
                    rows.push((key.clone(), all));
                } else {
                    let row: Fields = v
                        .fields
                        .iter()
                        .filter_map(|f| {
                            all.get(&f.name)
                                .filter(|x| **x != Node::Null)
                                .map(|x| (f.name.clone(), x.clone()))
                        })
                        .collect();
                    rows.push((key.clone(), row));
                }
            }
        }
        if let Some(agg) = &v.aggregation {
            rows = aggregate(&agg.group_by, &agg.functions, rows)?;
        }
        for r in v.order_by.iter().rev() {
            let desc = r.direction == Direction::Descending;
            let (mut present, absent): (Vec<_>, Vec<_>) = rows
                .into_iter()
                .partition(|(_, row)| lookup(row, &r.field).is_some());
            present.sort_by(|a, b| {
                let o = sort_key(lookup(&a.1, &r.field).unwrap())
                    .cmp(&sort_key(lookup(&b.1, &r.field).unwrap()));
                if desc {
                    o.reverse()
                } else {
                    o
                }
            });
            rows = present.into_iter().chain(absent).collect();
        }
        Ok(rows)
    }

    /// Delivers emitted events to their bindings, synchronously, through `engine`.
    pub fn deliver(
        &self,
        engine: &mut Engine<'_>,
        events: &[Emitted],
        forced: &BTreeMap<String, String>,
        depth: usize,
    ) -> (Vec<Emitted>, Vec<Invocation>) {
        let mut delivered = Vec::new();
        let mut invocations = Vec::new();
        if depth > 4 {
            return (delivered, invocations);
        }
        for ev in events {
            for (bn, b) in self.ir.bindings() {
                if b.cause.event().map(ToString::to_string).as_deref() != Some(ev.event.as_str()) {
                    continue;
                }
                let command = b.command.to_string();
                let mut inp = Fields::new();
                let mut note = None;
                for m in &b.mapping {
                    let v = match &m.value {
                        ResolvedMappingValue::EventField { field, .. } => {
                            ev.payload.get(field).cloned()
                        }
                        ResolvedMappingValue::Literal { value } => {
                            match self.literal(&m.target_type, value) {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    note = Some(e.0);
                                    None
                                }
                            }
                        }
                        _ => {
                            note = Some(format!("the binding value source of `{}`", m.target));
                            None
                        }
                    };
                    if let Some(v) = v {
                        inp.insert(m.target.clone(), v);
                    }
                }
                if let Some(note) = note {
                    invocations.push(Invocation {
                        binding: bn.to_string(),
                        command,
                        input: inp,
                        outcome: None,
                        error: None,
                        retry: false,
                        note: Some(note),
                    });
                    continue;
                }
                let step = match engine.execute(
                    &command,
                    &inp,
                    forced.get(&command).map(String::as_str),
                    None,
                ) {
                    Ok(s) => s,
                    Err(u) => {
                        invocations.push(Invocation {
                            binding: bn.to_string(),
                            command,
                            input: inp,
                            outcome: None,
                            error: None,
                            retry: false,
                            note: Some(u.0),
                        });
                        continue;
                    }
                };
                invocations.push(Invocation {
                    binding: bn.to_string(),
                    command: command.clone(),
                    input: inp.clone(),
                    outcome: step.outcome.clone(),
                    error: step.error.clone(),
                    retry: false,
                    note: None,
                });
                let mut last = step;
                if last.error.is_some() && b.failure == Failure::Retry {
                    // At-least-once delivery retries a refused invocation; a forced outcome answers once.
                    if let Ok(again) = engine.execute(&command, &inp, None, None) {
                        invocations.push(Invocation {
                            binding: bn.to_string(),
                            command: command.clone(),
                            input: inp.clone(),
                            outcome: again.outcome.clone(),
                            error: again.error.clone(),
                            retry: true,
                            note: None,
                        });
                        last = again;
                    }
                }
                delivered.extend(last.events.iter().cloned());
                if last.error.is_some() && b.failure == Failure::Escalate {
                    if let Some(esc) = &b.escalation {
                        let decl = self.ir.event(esc);
                        let names: Vec<String> =
                            decl.fields.iter().map(|f| f.name.clone()).collect();
                        delivered.push(Emitted {
                            event: decl.name.to_string(),
                            payload: names
                                .iter()
                                .filter_map(|k| inp.get(k).map(|v| (k.clone(), v.clone())))
                                .collect(),
                            open: names
                                .iter()
                                .filter(|k| !inp.contains_key(*k))
                                .cloned()
                                .collect(),
                            uncalled: Vec::new(),
                        });
                    }
                }
                let (more, inv) = self.deliver(engine, &last.events, forced, depth + 1);
                delivered.extend(more);
                invocations.extend(inv);
            }
        }
        (delivered, invocations)
    }
}

enum Val {
    Is(Option<Node>),
    Increment(String),
}

/// `pred` without the top-level conjuncts that read a parameter `bound` leaves unbound.
fn bind_params(pred: &Predicate, bound: &Fields) -> Result<Option<Predicate>, Undetermined> {
    let reads_unbound = |p: &Predicate| {
        p.fact_paths().iter().any(|path| {
            path.segments().first().is_some_and(|s| s == "param")
                && path
                    .segments()
                    .get(1)
                    .is_some_and(|n| !bound.contains_key(n))
        })
    };
    match pred {
        Predicate::All(children) => {
            let kept: Vec<Predicate> = children
                .iter()
                .filter(|c| !reads_unbound(c))
                .cloned()
                .collect();
            Ok(if kept.is_empty() {
                None
            } else {
                Some(Predicate::all(kept))
            })
        }
        p if !reads_unbound(p) => Ok(Some(p.clone())),
        Predicate::Compare { .. }
        | Predicate::AnyOf { .. }
        | Predicate::NoneOf { .. }
        | Predicate::Defined(_)
        | Predicate::Truthy(_) => Ok(None),
        p => undetermined(format!("a parameter read inside `{p}`")),
    }
}

/// The value at a dotted path of a row.
pub fn lookup<'a>(row: &'a Fields, path: &str) -> Option<&'a Node> {
    let mut parts = path.split('.');
    let mut cur = row.get(parts.next()?)?;
    for p in parts {
        cur = cur.as_map()?.get(p)?;
    }
    (*cur != Node::Null).then_some(cur)
}

/// How rows sort: numbers first, by value, then text.
#[derive(PartialEq)]
pub enum SortKey {
    /// A number.
    Num(f64),
    /// Anything else, as text.
    Text(String),
}

impl Eq for SortKey {}
impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.total_cmp(b),
            (Self::Num(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Num(_)) => Ordering::Greater,
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
        }
    }
}

/// The key a value sorts by.
pub fn sort_key(v: &Node) -> SortKey {
    match v {
        Node::Number(n) => SortKey::Num(n.get()),
        Node::Text(t) => SortKey::Text(t.clone()),
        other => SortKey::Text(serde_json::to_string(other).unwrap_or_default()),
    }
}

fn aggregate(
    keys: &[String],
    functions: &BTreeMap<String, ess_compiler::ir::ResolvedAggregate>,
    rows: Vec<(String, Fields)>,
) -> Result<Vec<(String, Fields)>, Undetermined> {
    let mut groups: BTreeMap<Vec<String>, Vec<Fields>> = BTreeMap::new();
    for (_, facts) in rows {
        let k: Vec<String> = keys
            .iter()
            .map(|x| serde_json::to_string(facts.get(x).unwrap_or(&Node::Null)).unwrap_or_default())
            .collect();
        groups.entry(k).or_default().push(facts);
    }
    if keys.is_empty() && groups.is_empty() {
        groups.insert(Vec::new(), Vec::new());
    }
    let mut out = Vec::new();
    for (k, members) in groups {
        let mut row: Fields = Fields::new();
        if let Some(first) = members.first() {
            for x in keys {
                row.insert(x.clone(), first.get(x).cloned().unwrap_or(Node::Null));
            }
        }
        for (name, f) in functions {
            let field = f.input.as_ref().map(|i| i.name.clone());
            let vals: Vec<&Node> = match &field {
                Some(field) => members
                    .iter()
                    .filter_map(|m| m.get(field))
                    .filter(|v| **v != Node::Null)
                    .collect(),
                None => Vec::new(),
            };
            let nums: Vec<f64> = vals
                .iter()
                .filter_map(|v| {
                    if let Node::Number(n) = v {
                        Some(n.get())
                    } else {
                        None
                    }
                })
                .collect();
            let num = |x: f64| {
                Number::new(x)
                    .map(Node::Number)
                    .map_err(|e| Undetermined(e.to_string()))
            };
            let v = match f.function {
                AggregateFunction::Count => Some(Node::Number(Number::from(members.len()))),
                AggregateFunction::CountDistinct => {
                    let mut seen: Vec<String> = vals
                        .iter()
                        .map(|v| serde_json::to_string(v).unwrap_or_default())
                        .collect();
                    seen.sort();
                    seen.dedup();
                    Some(Node::Number(Number::from(seen.len())))
                }
                AggregateFunction::Sum => Some(num(nums.iter().sum())?),
                AggregateFunction::Min | AggregateFunction::Max => {
                    let mut sorted: Vec<&Node> = vals.clone();
                    sorted.sort_by_key(|a| sort_key(a));
                    if f.function == AggregateFunction::Min {
                        sorted.first()
                    } else {
                        sorted.last()
                    }
                    .map(|v| (*v).clone())
                }
                AggregateFunction::Avg => {
                    if nums.is_empty() {
                        None
                    } else {
                        #[allow(clippy::cast_precision_loss)]
                        let n = nums.len() as f64;
                        Some(num(nums.iter().sum::<f64>() / n)?)
                    }
                }
            };
            if let Some(v) = v {
                row.insert(name.clone(), v);
            }
        }
        out.push((k.join("|"), row));
    }
    Ok(out)
}

// ---- ESS first --------------------------------------------------------------------------------

/// Executes commands through ESS's interpreter while it holds the page's store, and through the
/// page's interpreter otherwise.
pub struct Engine<'ir> {
    /// The page's interpreter.
    pub interp: &'ir Interp<'ir>,
    /// The page's world.
    pub world: World,
    ess: Option<ess_exec::Store>,
    /// Steps ESS answered.
    pub by_ess: usize,
    /// Steps the page answered.
    pub by_page: usize,
    /// Where ESS and the page read one step differently.
    pub divergences: Vec<String>,
}

impl<'ir> Engine<'ir> {
    /// A fresh run.
    pub fn new(interp: &'ir Interp<'ir>) -> Self {
        Self {
            interp,
            world: World::default(),
            ess: Some(ess_exec::Store::default()),
            by_ess: 0,
            by_page: 0,
            divergences: Vec::new(),
        }
    }

    /// What ESS's interpreter answers for `command` against the store it holds: the number of steps
    /// it allows, or why it determines none. `None` once ESS no longer holds the page's store.
    pub fn ask_ess_raw(&self, command: &str, input: &Fields) -> Option<Result<usize, String>> {
        let store = self.ess.as_ref()?;
        let (name, _) = self
            .interp
            .ir
            .commands()
            .iter()
            .find(|(k, _)| k.to_string() == command)?;
        Some(
            ess_exec::execute(
                self.interp.ir,
                store,
                name,
                input,
                &ess_exec::Externals::Withheld,
            )
            .map(|s| s.len())
            .map_err(|e| e.to_string()),
        )
    }

    /// Whether ESS still holds the page's store.
    pub fn in_step_with_ess(&self) -> bool {
        self.ess.is_some()
    }

    /// Executes one command and keeps its world.
    pub fn execute(
        &mut self,
        command: &str,
        input: &Fields,
        forced: Option<&str>,
        caller: Option<&Fields>,
    ) -> Result<Step, Undetermined> {
        let mine = self
            .interp
            .step(&self.world, command, input, forced, caller);
        let ess = self
            .ess
            .as_ref()
            .and_then(|store| self.ask_ess(store, command, input, forced, caller.is_some()));
        let step = match (ess, mine) {
            (Some((ess_step, next)), mine) => {
                if let Ok(m) = &mine {
                    let same = m.outcome == ess_step.outcome
                        && m.error == ess_step.error
                        && m.world.same_rows(&ess_step.world)
                        && m.events
                            .iter()
                            .map(|e| (&e.event, &e.payload))
                            .eq(ess_step.events.iter().map(|e| (&e.event, &e.payload)));
                    if !same {
                        self.divergences.push(format!(
                            "{command}: ESS answers {:?}/{:?}, the page {:?}/{:?}",
                            ess_step.outcome, ess_step.error, m.outcome, m.error
                        ));
                    }
                } else if let Err(e) = &mine {
                    self.divergences.push(format!(
                        "{command}: ESS answers {:?}, the page cannot: {e}",
                        ess_step.outcome
                    ));
                }
                self.ess = Some(next);
                self.by_ess += 1;
                let mut s = ess_step;
                if let Ok(m) = mine {
                    // The counter is the page's; ESS keeps its own and does not publish it.
                    s.world.minted = m.world.minted.max(s.world.minted);
                    s.note = m.note;
                    s.broken = m.broken;
                    s.refused = m.refused;
                    for (e, me) in s.events.iter_mut().zip(m.events) {
                        e.open = me.open;
                        e.uncalled = me.uncalled;
                    }
                }
                s
            }
            (None, mine) => {
                let m = mine?;
                if !m.world.same_rows(&self.world) {
                    self.ess = None;
                }
                self.by_page += 1;
                m
            }
        };
        self.world = step.world.clone();
        Ok(step)
    }

    fn ask_ess(
        &self,
        store: &ess_exec::Store,
        command: &str,
        input: &Fields,
        forced: Option<&str>,
        caller: bool,
    ) -> Option<(Step, ess_exec::Store)> {
        if caller {
            // ESS's interpreter has no caller; a step that names one is the page's.
            return None;
        }
        let ir = self.interp.ir;
        let (name, cmd) = ir
            .commands()
            .iter()
            .find(|(k, _)| k.to_string() == command)?;
        let externals = match forced {
            Some(f) => {
                let o = cmd
                    .outcomes
                    .iter()
                    .find(|o| o.name.to_string() == f && is_external(o))?;
                ess_exec::Externals::Forced(o.name.clone())
            }
            None => ess_exec::Externals::Withheld,
        };
        let mut steps = ess_exec::execute(ir, store, name, input, &externals).ok()?;
        if steps.len() != 1 {
            return None;
        }
        let s = steps.remove(0);
        let mut world = World {
            inst: BTreeMap::new(),
            minted: self.world.minted,
        };
        for (entity, key, inst) in s.next.instances() {
            let e = &ir.entities()[entity];
            let id = self
                .world
                .inst
                .get(&entity.to_string())
                .and_then(|h| h.get(key))
                .map(|i| i.id.clone())
                .or_else(|| {
                    s.events
                        .iter()
                        .find_map(|ev| ev.payload.values().find(|v| key_of(v) == key).cloned())
                })
                .unwrap_or_else(|| Node::Text(key.to_owned()));
            let _ = e;
            world.inst.entry(entity.to_string()).or_default().insert(
                key.to_owned(),
                Inst {
                    id,
                    state: inst.state.to_string(),
                    fields: inst.fields.clone(),
                },
            );
        }
        let events = s
            .events
            .iter()
            .map(|e| Emitted {
                event: e.event.to_string(),
                payload: e.payload.clone(),
                open: Vec::new(),
                uncalled: Vec::new(),
            })
            .collect();
        let outcome = s.outcome.as_ref().map(|o| o.outcome.to_string());
        let error = s.error.as_ref().map(|e| e.error.to_string());
        let refused = error.is_some() || outcome.is_none();
        Some((
            Step {
                outcome,
                error,
                events,
                world,
                refused,
                note: None,
                broken: None,
                open: Vec::new(),
                by: By::Ess,
            },
            s.next,
        ))
    }
}
