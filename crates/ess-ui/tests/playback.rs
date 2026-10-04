//! Playback against ESS's interpreter, and one test per construct the page executes beyond it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ess_compiler::EssIr;
use ess_conformance::interpret::execute as ess;
use ess_primitives::node::Node;
use ess_ui::interp::{By, Engine, Fields, Interp};
use serde_json::{json, Value};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn model(name: &str) -> EssIr {
    let d = fixture(name);
    ess_ui::typed_model(
        &d.join("spec"),
        &std::fs::read_to_string(d.join("ir.json")).unwrap(),
    )
    .unwrap()
}

fn input(v: Value) -> Fields {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, x)| (k.clone(), serde_json::from_value(x.clone()).unwrap()))
        .collect()
}

fn text(n: &Node) -> String {
    n.as_text().unwrap().to_owned()
}

/// Steps one authored scenario through the page and, beside it, through ESS's interpreter on a
/// store of its own: every command's outcome, error and every instance's state must agree, and the
/// state transitions in the step script the page plays must be the ones ESS's store went through.
#[test]
fn billing_scenario_plays_back_as_ess_executes_it() {
    let ir = model("billing");
    let interp = Interp::new(&ir);
    let suite: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("billing").join("authored.json")).unwrap(),
    )
    .unwrap();
    let (name, scenario) = suite["scenarios"]
        .as_object()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let (trace, engine) = ess_ui::replay::replay(&interp, scenario, true);
    assert!(engine.divergences.is_empty(), "{:?}", engine.divergences);
    let (script, _) = ess_ui::sim::script_of_trace(
        &interp,
        &trace,
        &ess_ui::sim::acceptors(&ir),
        &BTreeMap::new(),
        None,
    );

    let mut store = ess::Store::default();
    let mut captures: BTreeMap<String, Node> = BTreeMap::new();
    let mut executed = 0;
    let commands: BTreeMap<String, _> = ir
        .commands()
        .keys()
        .map(|k| (k.to_string(), k.clone()))
        .collect();
    let mut script_steps = script.iter().filter(|s| s["k"] == "execute_command");
    for (i, rec) in trace.iter().enumerate() {
        if rec.step == "capture_instance" {
            let ev = &trace[..i]
                .iter()
                .rev()
                .find(|r| r.step == "execute_command")
                .unwrap()
                .events;
            let hit = ev
                .iter()
                .find(|e| e.event == rec.raw["event"].as_str().unwrap())
                .unwrap();
            captures.insert(
                rec.raw["instance"].as_str().unwrap().to_owned(),
                hit.payload[rec.raw["field"].as_str().unwrap()].clone(),
            );
        }
        if rec.step != "execute_command" {
            continue;
        }
        let mut inp = Fields::new();
        for (k, v) in rec.raw["input"].as_object().unwrap() {
            let n = match v["kind"].as_str() {
                Some("instance") => captures[v["instance"].as_str().unwrap()].clone(),
                _ => serde_json::from_value(v["value"].clone()).unwrap(),
            };
            inp.insert(k.clone(), n);
        }
        assert_eq!(inp, rec.input, "{name}: the page sent another input");
        let cmd = &commands[rec.command.as_deref().unwrap()];
        let before: BTreeMap<String, String> = store
            .instances()
            .map(|(_, k, i)| (k.to_owned(), i.state.to_string()))
            .collect();
        let mut steps = ess::execute(&ir, &store, cmd, &inp, &ess::Externals::Withheld).unwrap();
        assert_eq!(steps.len(), 1);
        let mut step = steps.remove(0);
        // ESS does not deliver bindings; the harness does, through ESS, as the page does.
        for ev in step.events.clone() {
            for b in ir.bindings().values().filter(|b| {
                b.cause
                    .event()
                    .is_some_and(|e| e.to_string() == ev.event.to_string())
            }) {
                let mut bound = Fields::new();
                for m in &b.mapping {
                    match &m.value {
                        ess_compiler::ir::ResolvedMappingValue::EventField { field, .. } => {
                            bound.insert(m.target.clone(), ev.payload[field].clone());
                        }
                        ess_compiler::ir::ResolvedMappingValue::Literal { value } => {
                            bound.insert(m.target.clone(), Node::Text(value.clone()));
                        }
                        other => panic!("binding value {other:?}"),
                    }
                }
                let mut delivered = ess::execute(
                    &ir,
                    &step.next,
                    b.command.name(),
                    &bound,
                    &ess::Externals::Withheld,
                )
                .unwrap();
                assert_eq!(delivered.len(), 1);
                step.next = delivered.remove(0).next;
            }
        }
        assert_eq!(
            step.outcome.as_ref().map(|o| o.outcome.to_string()),
            rec.outcome,
            "{name}: outcome of {cmd}"
        );
        assert_eq!(
            step.error.as_ref().map(|e| e.error.to_string()),
            rec.error,
            "{name}: error of {cmd}"
        );
        assert!(rec.by_ess, "{name}: {cmd} was not answered by ESS");
        let after: BTreeMap<String, String> = step
            .next
            .instances()
            .map(|(_, k, i)| (k.to_owned(), i.state.to_string()))
            .collect();
        let page_states: BTreeMap<String, String> = rec
            .world
            .inst
            .values()
            .flat_map(|h| h.iter().map(|(k, i)| (k.clone(), i.state.clone())))
            .collect();
        assert_eq!(after, page_states, "{name}: states after {cmd}");
        // The step script the canvas plays moves exactly the instances ESS moved, between the same states.
        let shown = script_steps.next().unwrap();
        let mut moved: Vec<(String, Option<String>, Option<String>)> = shown["d"]
            .as_array()
            .map(|d| {
                d.iter()
                    .map(|x| {
                        (
                            x[1].as_str().unwrap().to_owned(),
                            x[2].as_str().map(ToOwned::to_owned),
                            x[3].as_str().map(ToOwned::to_owned),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        moved.retain(|(_, b, a)| b != a);
        let mut ess_moved: Vec<(String, Option<String>, Option<String>)> = after
            .iter()
            .filter(|(k, s)| before.get(*k) != Some(s))
            .map(|(k, s)| (k.clone(), before.get(k).cloned(), Some(s.clone())))
            .collect();
        ess_moved.sort();
        moved.sort();
        assert_eq!(
            moved, ess_moved,
            "{name}: transitions the canvas plays for {cmd}"
        );
        store = step.next;
        executed += 1;
    }
    assert_eq!(executed, 4);
}

/// The ESS-executable prefix of a scenario that needs the page's extensions: ESS answers every step
/// until the first one it does not execute, and the page agrees with it on each of them.
#[test]
fn parcel_scenario_agrees_with_ess_until_an_extension_is_needed() {
    let ir = model("parcel-locker");
    let interp = Interp::new(&ir);
    let suite: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("parcel-locker").join("authored.json")).unwrap(),
    )
    .unwrap();
    let scenario = suite["scenarios"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(k, _)| k.ends_with("deposit-and-collect"))
        .unwrap()
        .1;
    let (trace, engine) = ess_ui::replay::replay(&interp, scenario, true);
    assert!(engine.divergences.is_empty(), "{:?}", engine.divergences);
    let execs: Vec<_> = trace
        .iter()
        .filter(|r| r.step == "execute_command")
        .collect();
    assert!(execs[0].by_ess, "InstallLocker is ESS's");
    assert!(
        execs.iter().any(|r| !r.by_ess),
        "the scenario needs an extension"
    );
    assert!(
        trace.iter().all(|r| r.ok != Some(false)),
        "every expectation is met"
    );
}

fn engine(ir: &EssIr) -> (Interp<'_>, ()) {
    (Interp::new(ir), ())
}

fn gap(e: &Engine<'_>, cmd: &str, inp: &Fields) -> String {
    match e.ask_ess_raw(cmd, inp) {
        Some(Err(why)) => why,
        Some(Ok(n)) => panic!("ESS determines {cmd} ({n} step(s)); this is not an extension"),
        None => panic!("ESS no longer holds the store"),
    }
}

fn install(e: &mut Engine<'_>) -> Node {
    let s = e
        .execute(
            "parcels.locker.InstallLocker",
            &input(json!({"site": "Harbour"})),
            None,
            None,
        )
        .unwrap();
    s.events[0].payload["locker_id"].clone()
}

#[test]
fn existing_instance_refuses_an_identity_already_held() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    let inp = input(json!({"parcel_code": "PX-1", "locker_id": locker, "recipient": "Ada"}));
    assert!(gap(&e, "parcels.locker.DepositParcel", &inp).contains("existing-instance"));
    assert_eq!(
        e.execute("parcels.locker.DepositParcel", &inp, None, None)
            .unwrap()
            .outcome
            .as_deref(),
        Some("deposited")
    );
    let again = e
        .execute("parcels.locker.DepositParcel", &inp, None, None)
        .unwrap();
    assert_eq!(
        (again.outcome.as_deref(), again.error.as_deref()),
        (
            Some("already-deposited"),
            Some("parcels.locker.ParcelCodeTaken")
        )
    );
}

#[test]
fn creation_takes_the_identity_the_caller_supplies() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    let s = e
        .execute(
            "parcels.locker.DepositParcel",
            &input(json!({"parcel_code": "PX-7", "locker_id": locker, "recipient": "Ada"})),
            None,
            None,
        )
        .unwrap();
    assert_eq!(s.by, By::Page);
    assert!(e.world.inst["parcels.locker.Parcel"].contains_key("PX-7"));
    assert_eq!(
        e.world.inst["parcels.locker.Parcel"]["PX-7"].state,
        "Stored"
    );
}

#[test]
fn related_row_predicate_and_its_absence_are_read() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let missing = input(
        json!({"parcel_code": "PX-2", "locker_id": "00000000-0000-4000-8000-000000000999", "recipient": "Ada"}),
    );
    assert_eq!(
        e.execute("parcels.locker.DropParcel", &missing, None, None)
            .unwrap()
            .outcome
            .as_deref(),
        Some("no-locker")
    );
    let locker = install(&mut e);
    e.execute(
        "parcels.locker.CloseLocker",
        &input(json!({"locker_id": locker})),
        None,
        None,
    )
    .unwrap();
    let inp = input(json!({"parcel_code": "PX-2", "locker_id": locker, "recipient": "Ada"}));
    assert!(gap(&e, "parcels.locker.ReserveParcel", &inp).contains("related row"));
    let s = e
        .execute("parcels.locker.ReserveParcel", &inp, None, None)
        .unwrap();
    assert_eq!(s.outcome.as_deref(), Some("locker-closed"));
    let staff = input(json!({"parcel_code": "PX-2", "locker_id": locker, "recipient": "staff"}));
    assert_eq!(
        e.execute("parcels.locker.ReserveParcel", &staff, None, None)
            .unwrap()
            .outcome
            .as_deref(),
        Some("staff-closed"),
        "the input guard beside the related guard"
    );
}

#[test]
fn two_branches_that_hold_leave_the_step_open_first_declared_shown() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    let s = e
        .execute(
            "parcels.locker.RushParcel",
            &input(json!({"parcel_code": "PX-3", "locker_id": locker, "recipient": "vip"})),
            None,
            None,
        )
        .unwrap();
    let open: Vec<_> = s.open.iter().map(|(o, _)| o.clone().unwrap()).collect();
    assert_eq!(open, ["rush-a", "rush-b"]);
    assert_eq!(s.outcome.as_deref(), Some("rush-a"));
}

#[test]
fn subject_predicate_guards_held_fields_and_error_fields_read_the_row() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    e.execute(
        "parcels.locker.DepositParcel",
        &input(json!({"parcel_code": "PX-4", "locker_id": locker, "recipient": "Ada"})),
        None,
        None,
    )
    .unwrap();
    let before = e.world.clone();
    let urgent = input(json!({"parcel_code": "PX-4", "flag": "urgent"}));
    let s = e
        .execute("parcels.locker.FlagParcel", &urgent, None, None)
        .unwrap();
    assert_eq!(s.outcome.as_deref(), Some("flag-refused"));
    let fields = interp.error_fields(
        &before,
        "parcels.locker.FlagParcel",
        "flag-refused",
        &urgent,
        None,
    );
    assert_eq!(fields.get("recipient").map(text).as_deref(), Some("Ada"));
    assert_eq!(
        fields.get("site").map(text).as_deref(),
        Some("Harbour"),
        "a related row's field"
    );
    let plain = e
        .execute(
            "parcels.locker.FlagParcel",
            &input(json!({"parcel_code": "PX-4", "flag": "fragile"})),
            None,
            None,
        )
        .unwrap();
    assert_eq!(plain.outcome.as_deref(), Some("flagged"));
}

#[test]
fn subject_state_branches_read_the_held_state() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    e.execute(
        "parcels.locker.DepositParcel",
        &input(json!({"parcel_code": "PX-5", "locker_id": locker, "recipient": "Ada"})),
        None,
        None,
    )
    .unwrap();
    let urgent = input(json!({"parcel_code": "PX-5", "urgent": "Urgent"}));
    assert_eq!(
        e.execute("parcels.locker.TagParcel", &urgent, None, None)
            .unwrap()
            .outcome
            .as_deref(),
        Some("tagged")
    );
    e.execute(
        "parcels.locker.CollectParcel",
        &input(json!({"parcel_code": "PX-5"})),
        None,
        None,
    )
    .unwrap();
    assert_eq!(
        e.execute("parcels.locker.TagParcel", &urgent, None, None)
            .unwrap()
            .outcome
            .as_deref(),
        Some("tag-refused"),
        "a refusal naming no subject reads its siblings'"
    );
}

#[test]
fn a_string_identity_is_minted_and_skips_one_already_held() {
    let ir = model("parcel-locker");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let locker = install(&mut e);
    let held = "00000000-0000-4000-8000-000000000002";
    e.execute(
        "parcels.locker.RegisterParcel",
        &input(json!({"parcel_code": held, "locker_id": locker, "recipient": "Ada"})),
        None,
        None,
    )
    .unwrap();
    let s = e
        .execute(
            "parcels.locker.RegisterParcel",
            &input(json!({"locker_id": locker, "recipient": "Bo"})),
            None,
            None,
        )
        .unwrap();
    let code = text(&s.events[0].payload["parcel_code"]);
    assert!(
        code.starts_with("00000000-0000-4000-8000-") && code != held,
        "{code}"
    );
    assert_eq!(e.world.inst["parcels.locker.Parcel"].len(), 2);
}

#[test]
fn an_integer_identity_is_minted() {
    let ir = model("tally");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let inp = input(json!({"text": "Restock"}));
    assert!(gap(&e, "tally.board.PinNote", &inp).contains("mint"));
    let s = e.execute("tally.board.PinNote", &inp, None, None).unwrap();
    assert!(
        matches!(s.events[0].payload["note_no"], Node::Number(_)),
        "{:?}",
        s.events[0].payload
    );
}

#[test]
fn increment_adds_to_the_held_value_and_views_aggregate_and_rank() {
    let ir = model("tally");
    let (interp, ()) = engine(&ir);
    let mut e = Engine::new(&interp);
    let mut ids = Vec::new();
    for label in ["Coffee", "Tea"] {
        let s = e
            .execute(
                "tally.board.CreateCounter",
                &input(json!({"label": label, "team": "Red"})),
                None,
                None,
            )
            .unwrap();
        ids.push(s.events[0].payload["counter_id"].clone());
    }
    let bump = input(json!({"counter_id": ids[1]}));
    assert!(gap(&e, "tally.board.BumpCounter", &bump).contains("previous value plus"));
    e.execute("tally.board.BumpCounter", &bump, None, None)
        .unwrap();
    e.execute("tally.board.BumpCounter", &bump, None, None)
        .unwrap();
    let rows = interp
        .view_rows(&e.world, "tally.board.Counters", None)
        .unwrap();
    assert_eq!(
        rows[0].1["label"],
        Node::Text("Tea".into()),
        "ordered by hits desc"
    );
    assert_eq!(rows[0].1["hits"].to_string(), "2");
    let totals = interp
        .view_rows(&e.world, "tally.board.HitsByTeam", None)
        .unwrap();
    assert_eq!(totals.len(), 1);
    assert_eq!(totals[0].1["counters"].to_string(), "2");
    assert_eq!(totals[0].1["hits"].to_string(), "2");
}

#[test]
fn bindings_deliver_emitted_events_with_retry_and_invocations() {
    let ir = model("billing");
    let interp = Interp::new(&ir);
    let suite: Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("billing").join("suite.json")).unwrap(),
    )
    .unwrap();
    let (traces, tally, divergences) = ess_ui::replay::replay_suite(&interp, &suite, false);
    assert!(divergences.is_empty());
    assert_eq!(
        tally["expect_invocation"],
        [1, 1, 0],
        "the binding's invocation is observed"
    );
    assert_eq!(tally["redeliver_event"], [1, 1, 0]);
    assert!(traces.values().flatten().any(|r| !r.invocations.is_empty()));
    assert!(ess_ui::replay::failures(&traces).is_empty());
}
