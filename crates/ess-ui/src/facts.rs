//! Fact sources the page's guards are decided over, built from ESS's own input binder.
//!
//! Every guard, invariant and view filter is a parsed [`Predicate`] from the compiled model and is
//! decided by ESS's evaluator (`Predicate::evaluate`). This module only assembles the facts: the
//! held row of an entity (its fields, its identity and `state`), the command's input under
//! `input.`, and a view's parameters under `param.`. Each part is projected by
//! [`ess_conformance::input::bind`], so a value is read exactly as ESS reads a command's input.

use std::collections::BTreeMap;

use ess_compiler::ir::{EssIr, ResolvedField, ResolvedTypeRef};
use ess_conformance::input::{bind, Completeness};
use ess_domain::types::Primitive;
use ess_primitives::facts::{FactPath, FactSource, FactStore, FactValue, Scales};
use ess_primitives::node::Node;
use ess_primitives::predicate::{Predicate, Truth};

/// One namespace of facts: the fields that declare it, under an optional first path segment.
struct Scope {
    prefix: Option<String>,
    fields: Vec<ResolvedField>,
}

/// Facts read beside the fields that declare them, so a declared `Timestamp` orders by instant.
pub struct Facts<'a> {
    ir: &'a EssIr,
    store: FactStore,
    scopes: Vec<Scope>,
}

impl<'a> Facts<'a> {
    /// No facts.
    pub fn new(ir: &'a EssIr) -> Self {
        let mut store = FactStore::new();
        store.order_text_by_bytes();
        Self {
            ir,
            store,
            scopes: Vec::new(),
        }
    }

    /// Binds `values` against `fields`, under `prefix` when one is given.
    ///
    /// A value the declared type does not admit is reported, as ESS reports it for an input.
    pub fn bind(
        &mut self,
        prefix: Option<&str>,
        fields: &[ResolvedField],
        values: &BTreeMap<String, Node>,
    ) -> Result<(), String> {
        let known: BTreeMap<String, Node> = values
            .iter()
            .filter(|(name, value)| {
                **value != Node::Null && fields.iter().any(|field| &field.name == *name)
            })
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        let bound =
            bind(self.ir, fields, &known, Completeness::Partial).map_err(|e| e.to_string())?;
        for (path, value) in bound.iter() {
            self.store.set(prefixed(prefix, path), value.clone());
        }
        for name in known.keys() {
            if let Ok(path) = FactPath::new(name) {
                self.store.mark_present(prefixed(prefix, &path));
            }
        }
        self.scopes.push(Scope {
            prefix: prefix.map(ToOwned::to_owned),
            fields: fields.to_vec(),
        });
        Ok(())
    }

    /// Binds one text fact, such as `state`.
    pub fn text(&mut self, path: &str, value: &str) {
        if let Ok(path) = FactPath::new(path) {
            self.store.set(path, FactValue::text(value));
        }
    }

    /// Decides `predicate`: `Some(true)`, `Some(false)`, or `None` for Unknown.
    pub fn decide(&self, predicate: &Predicate) -> Option<bool> {
        truth(predicate.evaluate(self))
    }
}

/// `Truth` as an optional boolean: Unknown is `None`.
pub fn truth(value: Truth) -> Option<bool> {
    match value {
        Truth::True => Some(true),
        Truth::False => Some(false),
        Truth::Unknown => None,
    }
}

fn prefixed(prefix: Option<&str>, path: &FactPath) -> FactPath {
    match prefix {
        None => path.clone(),
        Some(prefix) => {
            let mut segments = vec![prefix.to_owned()];
            segments.extend(path.segments().iter().cloned());
            FactPath::from_segments(segments)
        }
    }
}

impl Facts<'_> {
    fn declared(&self, path: &FactPath) -> Option<Primitive> {
        for scope in &self.scopes {
            let rest = match &scope.prefix {
                None => path.clone(),
                Some(prefix) => {
                    let segments = path.segments();
                    if segments.len() < 2 || &segments[0] != prefix {
                        continue;
                    }
                    FactPath::from_segments(segments[1..].to_vec())
                }
            };
            if let Ok(resolved) = ess_compiler::expression::resolve_path(
                self.ir,
                &scope.fields,
                &rest,
                "ess-ui facts",
            ) {
                if let ResolvedTypeRef::Primitive { name } = resolved.terminal {
                    return Some(name);
                }
                return None;
            }
        }
        None
    }
}

impl FactSource for Facts<'_> {
    fn fact(&self, path: &FactPath) -> Option<FactValue> {
        self.store.fact(path)
    }

    fn present(&self, path: &FactPath) -> bool {
        self.store.present(path)
    }

    fn scales(&self) -> &Scales {
        self.store.scales()
    }

    fn orders_as_instant(&self, path: &FactPath) -> bool {
        self.declared(path) == Some(Primitive::Timestamp)
    }

    fn orders_text_by_bytes(&self, path: &FactPath) -> bool {
        self.declared(path) != Some(Primitive::Duration)
    }
}
