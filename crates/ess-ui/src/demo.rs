//! The Demo control panel, and the Model sections that report on the simulation (overview,
//! simulation, changes).
//!
//! The canvas draws every entity's lifecycle in the visual language of the Model tab's diagrams,
//! smaller, in columns by owning component, with relations as faint links. The page plays the step
//! scripts [`crate::sim`] baked; nothing here decides a step.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;

use crate::model::{arr, g, short, st, strs, Model};
use crate::render::{clip_rect, esc, ranks, slug, speed_control, text_w};

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn machine(m: &Model<'_>, ename: &str, ox: f64, oy: f64) -> (String, f64, f64) {
    let ent = &m.entities[ename];
    let lc = g(ent, "lifecycle");
    let states = strs(lc, "states");
    let init = st(lc, "initial").to_owned();
    let term: BTreeSet<String> = strs(lc, "terminal").into_iter().collect();
    let mut edges = Vec::new();
    for t in arr(lc, "transitions") {
        for f in strs(t, "from") {
            if states.contains(&f) && states.iter().any(|s| s == st(t, "to")) {
                edges.push((f, st(t, "to").to_owned(), st(t, "name").to_owned()));
            }
        }
    }
    let (rank, col, _) = ranks(&states, &init, &term, &edges);
    let (colg, rankg, nh, head) = (168.0, 58.0, 26.0, 44.0);
    let nodew: BTreeMap<&String, f64> = states
        .iter()
        .map(|s| (s, (text_w(s, 7.2) + 22.0).max(86.0)))
        .collect();
    let minc = col.values().copied().min().unwrap_or(0) as f64;
    let maxw = nodew.values().copied().fold(86.0_f64, f64::max);
    let pad = 26.0;
    let x0 = ox + pad + maxw / 2.0 - minc * colg;
    let pos: BTreeMap<&String, (f64, f64)> = states
        .iter()
        .map(|s| {
            (
                s,
                (
                    x0 + col[s] as f64 * colg,
                    oy + head + 20.0 + rank[s] as f64 * rankg,
                ),
            )
        })
        .collect();
    let maxc = col.values().copied().max().unwrap_or(0) as f64;
    let right = x0 + maxc * colg + maxw / 2.0 + 24.0;
    let pairs: BTreeSet<(&String, &String)> = edges.iter().map(|(a, b, _)| (a, b)).collect();
    let mut lanes: Vec<(f64, f64, i64)> = Vec::new();
    let mut parts = Vec::new();
    let dom = st(ent, "domain");
    for (a, b, name) in &edges {
        let ((ax, ay), (bx, by)) = (pos[a], pos[b]);
        let (dr, dc) = (rank[b] - rank[a], col[b] - col[a]);
        let pid = format!("dm-{}-{}-{}-{}", slug(ename), slug(name), slug(a), slug(b));
        let (d, lx, ly);
        if a == b {
            d = format!(
                "M{:.1},{:.1} c40,-26 40,38 0,12",
                ax + nodew[a] / 2.0,
                ay - 6.0
            );
            lx = ax + nodew[a] / 2.0 + 34.0;
            ly = ay;
        } else if dr.abs() <= 1 && !(dr == 0 && dc.abs() > 1) {
            let (sx, sy) = clip_rect(ax, ay, nodew[a] + 4.0, nh + 4.0, bx, by);
            let (tx, ty) = clip_rect(bx, by, nodew[b] + 4.0, nh + 4.0, ax, ay);
            let bend = if pairs.contains(&(b, a)) { 16.0 } else { 0.0 };
            let l = ((tx - sx).powi(2) + (ty - sy).powi(2)).sqrt().max(1.0);
            let (px, py) = (-(ty - sy) / l, (tx - sx) / l);
            let (cx, cy) = ((sx + tx) / 2.0 + px * bend, (sy + ty) / 2.0 + py * bend);
            d = format!("M{sx:.1},{sy:.1} Q{cx:.1},{cy:.1} {tx:.1},{ty:.1}");
            lx = 0.25 * sx + 0.5 * cx + 0.25 * tx + if dc == 0 { 6.0 } else { 0.0 };
            ly = 0.25 * sy + 0.5 * cy + 0.25 * ty;
        } else {
            let (lo, hi) = if ay < by { (ay, by) } else { (by, ay) };
            let mut lane = 0;
            while lanes
                .iter()
                .any(|(l0, l1, ln)| *ln == lane && !(hi < l0 - 16.0 || lo > l1 + 16.0))
            {
                lane += 1;
            }
            lanes.push((lo, hi, lane));
            let x = right + lane as f64 * 16.0;
            let (sx, tx) = (ax + nodew[a] / 2.0, bx + nodew[b] / 2.0);
            let dy = if by > ay { 1.0 } else { -1.0 };
            d = format!(
                "M{sx:.1},{ay:.1} L{:.1},{ay:.1} Q{x:.1},{ay:.1} {x:.1},{:.1} L{x:.1},{:.1} Q{x:.1},{by:.1} {:.1},{by:.1} L{tx:.1},{by:.1}",
                x - 8.0,
                ay + dy * 8.0,
                by - dy * 8.0,
                x - 8.0
            );
            lx = x + 4.0;
            ly = (ay + by) / 2.0;
        }
        let causes = m.causes.get(&(ename.to_owned(), name.clone()));
        let kind = match causes {
            Some(cs) if !cs.is_empty() => ["guarded", "default", "unconditional"]
                .into_iter()
                .find(|k| cs.iter().any(|c| c.kind == *k))
                .unwrap_or("other"),
            _ => "none",
        };
        let cmds: BTreeSet<&str> = causes
            .into_iter()
            .flatten()
            .map(|c| c.command.as_str())
            .collect();
        parts.push(format!(
            r#"<path class="edge k-{kind}" data-p="{pid}" data-cmds="{}" d="{d}" marker-end="url(#arrow-{kind})"/>"#,
            esc(&cmds.into_iter().collect::<Vec<_>>().join(" "))
        ));
        parts.push(format!(
            r#"<text class="dm-tl" data-tl="{pid}" x="{lx:.1}" y="{:.1}">{}</text>"#,
            ly + 3.0,
            esc(name)
        ));
    }
    let mut nodes = Vec::new();
    for s in &states {
        let (x, y) = pos[s];
        let w = nodew[s];
        let mut cls = vec!["state"];
        if *s == init {
            cls.push("initial");
        }
        if term.contains(s) {
            cls.push("terminal");
        }
        let ring = if term.contains(s) {
            format!(
                r#"<rect class="ring" x="{:.1}" y="{:.1}" width="{:.1}" height="{}" rx="{}"/>"#,
                x - w / 2.0 - 4.0,
                y - nh / 2.0 - 4.0,
                w + 8.0,
                nh + 8.0,
                nh / 2.0 + 4.0
            )
        } else {
            String::new()
        };
        nodes.push(format!(
            r#"<g class="{}" data-ms="{}#{}" data-cx="{x:.1}" data-cy="{y:.1}">{ring}<rect class="body" x="{:.1}" y="{:.1}" width="{w:.1}" height="{nh}" rx="{}"/><text x="{x:.1}" y="{:.1}" text-anchor="middle">{}</text></g>"#,
            cls.join(" "), esc(ename), esc(s), x - w / 2.0, y - nh / 2.0, nh / 2.0, y + 4.0, esc(s)
        ));
    }
    let lanes_used: BTreeSet<i64> = lanes.iter().map(|l| l.2).collect();
    let mut xs: Vec<f64> = states.iter().map(|s| pos[s].0 + nodew[s] / 2.0).collect();
    xs.push(right + 16.0 * lanes_used.len() as f64);
    let ys: Vec<f64> = states.iter().map(|s| pos[s].1 + nh / 2.0).collect();
    let maxx = xs.iter().copied().fold(ox + 200.0, f64::max);
    let width = (maxx - ox + pad)
        .max(text_w(short(ename), 8.0) + 80.0)
        .max(220.0);
    let height = ys.iter().copied().fold(oy + head + 40.0, f64::max) - oy + 26.0;
    let ea = m.a("entity", ename);
    let frame = format!(
        r##"<g class="dm-frame" data-me="{}" data-dom="{}" style="--dc:{}" data-x="{ox}" data-y="{oy}" data-w="{width:.0}" data-h="{height:.0}"><rect class="dm-box" x="{ox}" y="{oy}" width="{width:.0}" height="{height:.0}" rx="14"/><rect class="dstripe" x="{ox}" y="{}" width="4" height="{:.0}" rx="2"/><text class="dm-name" x="{}" y="{}"><a href="#{}" data-model-ref="{}">{}</a></text><text class="dm-sub" x="{}" y="{}">{}</text>"##,
        esc(ename),
        esc(dom),
        m.dom_colour.get(dom).map_or("#888", String::as_str),
        oy + 12.0,
        height - 24.0,
        ox + 16.0,
        oy + 24.0,
        esc(&ea),
        esc(&ea),
        esc(short(ename)),
        ox + 16.0,
        oy + 38.0,
        esc(dom)
    );
    (
        format!("{frame}{}{}</g>", parts.concat(), nodes.concat()),
        width,
        height,
    )
}

fn render_canvas(m: &Model<'_>) -> String {
    let mut cols: Vec<(Option<String>, Vec<String>)> = Vec::new();
    for c in &m.comp_order {
        let mut ents = Vec::new();
        for d in strs(&m.comps[c], "owns") {
            ents.extend(
                m.domains
                    .get(&d)
                    .map(|dm| strs(dm, "entities"))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|e| m.entities.contains_key(e)),
            );
        }
        cols.push((Some(c.clone()), ents));
    }
    let placed: BTreeSet<String> = cols.iter().flat_map(|(_, es)| es.clone()).collect();
    let rest: Vec<String> = m
        .entities
        .keys()
        .filter(|e| !placed.contains(*e))
        .cloned()
        .collect();
    if !rest.is_empty() {
        cols.push((None, rest));
    }
    let mut parts = Vec::new();
    let mut heads = Vec::new();
    let mut boxes: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
    let mut x = 30.0;
    let mut h_max: f64 = 0.0;
    for (c, ents) in &cols {
        let mut y = 70.0;
        let mut colw: f64 = 240.0;
        for e in ents {
            let (svg, w, h) = machine(m, e, x, y);
            parts.push(svg);
            boxes.insert(e.clone(), (x, y, w, h));
            colw = colw.max(w);
            y += h + 30.0;
        }
        let cn = c.clone().unwrap_or_default();
        heads.push(format!(
            r#"<g class="dm-col" data-comp="{}"><rect class="dm-colbox" x="{}" y="16" width="{:.0}" height="{:.0}" rx="18"/><text class="dm-comp" x="{x}" y="46" data-comp-head="{}">{}</text></g>"#,
            esc(&cn), x - 12.0, colw + 24.0, y - 16.0, esc(&cn), esc(c.as_deref().unwrap_or("no owning component"))
        ));
        h_max = h_max.max(y);
        x += colw + 60.0;
    }
    let mut links = Vec::new();
    for (e, eo) in m.entities {
        for r in arr(eo, "relations") {
            let t = st(r, "target");
            let (Some(sb), Some(tb)) = (boxes.get(e), boxes.get(t)) else {
                continue;
            };
            let ((sx, sy, sw, _), (tx, ty, tw, _)) = (*sb, *tb);
            let k = if st(r, "kind") == "owns" {
                "own"
            } else {
                "ref"
            };
            let title = format!(
                "{} {} {} → {}",
                esc(e),
                esc(st(r, "kind")),
                esc(st(r, "name")),
                esc(t)
            );
            if (sx - tx).abs() < 1.0 {
                let (x1, y1, x2, y2) = (sx + sw, sy + 30.0, tx + tw, ty + 30.0);
                links.push(format!(r#"<path class="dm-rel k-{k}" d="M{x1:.0},{y1:.0} C{:.0},{y1:.0} {:.0},{y2:.0} {x2:.0},{y2:.0}"><title>{title}</title></path>"#, x1 + 60.0, x2 + 60.0));
            } else {
                let (x1, y1) = if tx > sx {
                    (sx + sw, sy + 30.0)
                } else {
                    (sx, sy + 30.0)
                };
                let (x2, y2) = if tx > sx {
                    (tx, ty + 30.0)
                } else {
                    (tx + tw, ty + 30.0)
                };
                let mx = (x1 + x2) / 2.0;
                links.push(format!(r#"<path class="dm-rel k-{k}" d="M{x1:.0},{y1:.0} C{mx:.0},{y1:.0} {mx:.0},{y2:.0} {x2:.0},{y2:.0}"><title>{title}</title></path>"#));
            }
        }
    }
    format!(
        r#"<svg id="dm-canvas" class="lifecycle dm-canvas" viewBox="0 0 {x:.0} {:.0}" data-full="0 0 {x:.0} {:.0}" role="img" aria-label="All entity lifecycles"><g class="dm-body">{}<g class="dm-links">{}</g>{}</g><g id="dm-tokens"></g></svg>"#,
        h_max + 20.0,
        h_max + 20.0,
        heads.concat(),
        links.concat(),
        parts.concat()
    )
}

/// The Demo section.
pub fn render_demo(m: &Model<'_>, sim: &Value, colour: &BTreeMap<String, String>) -> String {
    let opts = run_picker(sim);
    let actors: String = m
        .actors
        .iter()
        .map(|(a, ao)| {
            format!(
                r#"<button class="dm-actor" type="button" data-actor="{}" data-dom="{}" style="--c:{}" title="{}"><b>{}</b><span>{} commands</span></button>"#,
                esc(a), esc(st(ao, "domain")), colour.get(a).map_or("#888", String::as_str), esc(a), esc(short(a)), arr(ao, "may").len()
            )
        })
        .collect();
    let actors = if actors.is_empty() {
        r#"<div class="dim">The IR declares no actors; commands run without one.</div>"#.to_owned()
    } else {
        actors
    };
    let doms: String = m
        .domains
        .keys()
        .map(|d| format!(r#"<option value="{}">{}</option>"#, esc(d), esc(d)))
        .collect();
    format!(
        r#"<section id="demo" class="demo" aria-label="Demo control panel">
<div class="dm-top">
 {opts}
 <span class="dm-btns"><button type="button" data-dm="first" title="Start">|◀</button><button type="button" data-dm="prev" title="Step back">◀</button>
 <button type="button" data-dm="play" id="dm-play" title="Play or pause (space)">Play</button><button type="button" data-dm="next" title="Step (→)">▶</button></span>
 <input id="dm-scrub" type="range" min="0" max="0" value="0" aria-label="Timeline">
 {}
 <span class="dm-clock"><span id="dm-time"></span>step <b id="dm-step">0</b>/<span id="dm-n">0</span></span>
 <span class="dm-view"><label class="dm-follow"><input type="checkbox" id="dm-follow" checked> follow</label>
 <button type="button" data-dm="fit" title="Fit the canvas">Fit</button></span>
</div>
<div class="dm-rail" aria-label="Actors"><h4>actors</h4>{actors}</div>
<div class="dm-stage"><div class="dm-caption" id="dm-caption" aria-live="polite"></div>{}
 <div class="dm-pass" id="dm-pass" hidden></div></div>
<div class="dm-dock">
 <div class="dm-tabs" role="tablist">
  <button type="button" role="tab" data-dock="views" class="on">Views</button><button type="button" role="tab" data-dock="events">Events</button>
  <button type="button" role="tab" data-dock="seq">Sequence</button><button type="button" role="tab" data-dock="checks">Checks</button>
  <button type="button" role="tab" data-dock="inst">Inspector</button></div>
 <div class="dm-panel" data-panel="views" id="dm-views"></div>
 <div class="dm-panel" data-panel="events" hidden><label class="lbl">domain <select id="dm-evdom"><option value="">all</option>{doms}</select></label><ol id="dm-events" class="dm-feed"></ol></div>
 <div class="dm-panel" data-panel="seq" hidden><svg id="dm-seq" class="dm-seq" role="img" aria-label="Sequence diagram"></svg></div>
 <div class="dm-panel" data-panel="checks" hidden><ol id="dm-checks" class="dm-feed"></ol></div>
 <div class="dm-panel" data-panel="inst" hidden><div id="dm-inst" class="dim">Click a token on the canvas.</div></div>
</div>
<div class="dm-bottom"><svg id="dm-lanes" class="dm-lanes" role="img" aria-label="Instance swimlanes"></svg></div>
<svg id="dm-overlay" class="dm-overlay" aria-hidden="true"></svg>
</section>"#,
        speed_control("dm-speed"),
        render_canvas(m)
    )
}

fn srcnote(t: &str) -> String {
    format!(r#"<span class="srcnote">{}</span>"#, esc(t))
}

/// Gate results read from the build's logs.
#[derive(Default)]
pub struct Gate {
    /// Synthesized scenarios, refusals, and the log line.
    pub synth: Option<(u64, u64, String)>,
    /// The guard analysis's last line, exit code, and log line.
    pub guards: Option<(String, Option<i64>, String)>,
    /// Where the synthesized suite came from.
    pub suite_src: Option<String>,
    /// Where the authored scenarios came from.
    pub authored_src: Option<String>,
}

/// Parses the synthesis and guard-analysis logs the build passed in.
pub fn parse_gate(
    synth_log: Option<(&str, &str)>,
    guards_log: Option<(&str, &str)>,
    suite_src: Option<String>,
    authored_src: Option<String>,
) -> Gate {
    let mut out = Gate {
        suite_src,
        authored_src,
        ..Gate::default()
    };
    if let Some((path, t)) = synth_log {
        for (i, line) in t.lines().enumerate() {
            let nums: Vec<u64> = line
                .split_whitespace()
                .filter_map(|w| w.parse().ok())
                .collect();
            if line.contains("scenario(s)") && line.contains("refusal(s)") && nums.len() >= 2 {
                let sc = line
                    .split("scenario(s)")
                    .next()
                    .and_then(|p| p.split_whitespace().last())
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(nums[0]);
                let rf = line
                    .split("refusal(s)")
                    .next()
                    .and_then(|p| p.split_whitespace().last())
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(nums[1]);
                out.synth = Some((sc, rf, format!("{path}:{}", i + 1)));
            }
        }
    }
    if let Some((path, t)) = guards_log {
        let mut ex = None;
        let mut last = None;
        for (i, line) in t.lines().enumerate() {
            let s = line.trim();
            if let Some(code) = s.strip_prefix("exit ").and_then(|c| c.parse().ok()) {
                ex = Some(code);
            } else if !s.is_empty() {
                last = Some((s.to_owned(), i + 1));
            }
        }
        if let Some((line, li)) = last {
            out.guards = Some((line, ex, format!("{path}:{li}")));
        }
    }
    out
}

/// The Overview section.
pub fn render_overview(counts: &[(&str, usize)], gate: &Gate, sim: &Value) -> String {
    let cards: String = counts
        .iter()
        .map(|(k, v)| {
            format!(
                r#"<div class="ov"><b>{v}</b><span>{}</span>{}</div>"#,
                esc(k),
                srcnote("IR")
            )
        })
        .collect();
    let mut gs = Vec::new();
    if let Some((sc, rf, src)) = &gate.synth {
        gs.push(format!(
            r#"<div class="ov"><b>{sc}</b><span>synthesized scenarios</span>{}</div>"#,
            srcnote(src)
        ));
        gs.push(format!(
            r#"<div class="ov{}"><b>{rf}</b><span>synthesis refusals</span>{}</div>"#,
            if *rf > 0 { " warn" } else { "" },
            srcnote(src)
        ));
    }
    if let Some((line, ex, src)) = &gate.guards {
        gs.push(format!(
            r#"<div class="ov wide{}"><b>{}</b><span>guard analysis, exit {}</span>{}</div>"#,
            if ex.is_some_and(|e| e != 0) {
                " warn"
            } else {
                ""
            },
            esc(line),
            ex.map_or_else(|| "none".into(), |e| e.to_string()),
            srcnote(src)
        ));
    }
    for (label, key, src) in [
        ("synthesized suite", "tally", &gate.suite_src),
        ("authored scenarios", "authored_tally", &gate.authored_src),
    ] {
        let Some(tally) = sim
            .get(key)
            .and_then(Value::as_object)
            .filter(|t| !t.is_empty())
        else {
            continue;
        };
        let cnt = |v: &Value, i: usize| v.get(i).and_then(Value::as_u64).unwrap_or(0);
        let exp: Vec<&Value> = tally
            .iter()
            .filter(|(k, _)| k.starts_with("expect") || k.starts_with("eventually"))
            .map(|(_, v)| v)
            .collect();
        let met: u64 = exp.iter().map(|v| cnt(v, 0)).sum();
        let tot: u64 = exp.iter().map(|v| cnt(v, 1)).sum();
        let ne: u64 = tally.values().map(|v| cnt(v, 2)).sum();
        gs.push(format!(
            r#"<div class="ov{}"><b>{met} / {tot}</b><span>interpreter replay, {}{}</span>{}</div>"#,
            if met == tot { "" } else { " warn" }, esc(label), if ne > 0 { format!(", {ne} not evaluable") } else { String::new() },
            srcnote(&src.as_ref().map(|s| format!("{s} through ess-ui")).unwrap_or_default())
        ));
    }
    if let Some(e) = sim.get("engine") {
        let ess = e.get("ess").and_then(Value::as_u64).unwrap_or(0);
        let page = e.get("page").and_then(Value::as_u64).unwrap_or(0);
        let div = arr(e, "divergences").len();
        gs.push(format!(
            r#"<div class="ov{}"><b>{ess} / {}</b><span>commands executed by ESS's interpreter; the rest by the page's extensions{}</span>{}</div>"#,
            if div > 0 { " warn" } else { "" }, ess + page, if div > 0 { format!(", {div} divergence(s)") } else { String::new() }, srcnote("ess-conformance interpret::execute")
        ));
    }
    if gs.is_empty() {
        gs.push(r#"<p class="dim">No gate outputs were passed to the generator (<code>--suite</code>, <code>--scenarios</code>, <code>--synth-log</code>, <code>--guards-log</code>).</p>"#.into());
    }
    format!(
        r#"<h3 class="dom-h">declarations</h3><div class="ovgrid">{cards}</div><h3 class="dom-h">gate results read from the build</h3><div class="ovgrid">{}</div>"#,
        gs.concat()
    )
}

/// The Simulation section.
pub fn render_simulation(m: &Model<'_>, sim: &Value) -> String {
    let mut rows = String::new();
    for mt in arr(sim, "metrics") {
        let per: String = mt
            .get("per_actor")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(a, k)| {
                format!(
                    r#"<span class="chip" title="{}">{} {k}</span>"#,
                    esc(a),
                    esc(short(a))
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let f = |k: &str| mt.get(k).map(ToString::to_string).unwrap_or_default();
        let _ = write!(
            rows,
            r#"<tr><td class="mono">seed {}</td><td>{}</td><td>{}</td><td>{}</td><td>{} / {}</td><td>{} / {}</td><td>{per}</td><td><button type="button" data-open-trace="run:{}">Play</button></td></tr>"#,
            f("seed"),
            f("steps"),
            f("accepted"),
            f("refused"),
            f("transitions"),
            f("transitions_of"),
            f("outcomes"),
            f("outcomes_of"),
            f("seed")
        );
    }
    let un = strs(sim, "unreached");
    let unl: String = un
        .iter()
        .map(|o| {
            let a = m.a("outcome", o);
            format!(
                r##"<li><a class="ref mono" href="#{}" data-ref="{}">{}</a></li>"##,
                esc(&a),
                esc(&a),
                esc(o)
            )
        })
        .collect();
    let table = |key: &str| -> String {
        sim.get(key)
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .map(|(k, v)| {
                let c = |i: usize| v.get(i).and_then(Value::as_u64).unwrap_or(0);
                format!(
                    r#"<tr><td class="mono">{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"#,
                    esc(k),
                    c(0),
                    c(1),
                    c(2)
                )
            })
            .collect()
    };
    let tt = table("tally");
    let at = table("authored_tally");
    let notes: String = strs(sim, "notes")
        .iter()
        .map(|n| format!("<li>{}</li>", esc(n)))
        .collect();
    let mut out = format!(
        r#"<p class="lede">Every step on the Demo canvas was executed at build time: by ESS's own interpreter (<code>ess-conformance</code> <code>interpret::execute</code>) wherever it determines the step, and by this page's extensions of it where it does not yet. The page only plays the steps back. Seeded runs choose among the commands each actor may send, with inputs built only from values ESS gives (held identities, identities minted from the seed, enum variants, examples, suite literals).</p><table class="srcidx"><tr><th>run</th><th>steps</th><th>accepted</th><th>refused</th><th>transitions reached</th><th>outcomes reached</th><th>commands per actor</th><th></th></tr>{rows}</table><h3 class="dom-h">declared outcomes no seed reached ({})</h3><ul class="unr cols">{}</ul>"#,
        un.len(),
        if unl.is_empty() {
            "<li>none</li>".into()
        } else {
            unl
        }
    );
    if !tt.is_empty() {
        let _ = write!(
            out,
            r#"<h3 class="dom-h">replay of the synthesized suite, per step kind (met / evaluated / not evaluable)</h3><table class="srcidx"><tr><th>step</th><th>met</th><th>evaluated</th><th>not evaluable</th></tr>{tt}</table>"#
        );
    }
    if !at.is_empty() {
        let _ = write!(
            out,
            r#"<h3 class="dom-h">replay of the authored scenarios</h3><table class="srcidx"><tr><th>step</th><th>met</th><th>evaluated</th><th>not evaluable</th></tr>{at}</table>"#
        );
    }
    if !notes.is_empty() {
        let _ = write!(out, r#"<ul class="unr notes">{notes}</ul>"#);
    }
    out
}

/// `{(kind, id): added|changed}` and the change list, from an `ess verify diff --format json` document.
pub fn diff_index(diff: Option<&Value>) -> (BTreeMap<(String, String), String>, Vec<[String; 5]>) {
    let mut marks = BTreeMap::new();
    let mut items = Vec::new();
    for ch in diff.map(|d| arr(d, "changes")).unwrap_or(&[]) {
        let c = g(ch, "change");
        let (cat, subj, kind) = (
            st(c, "category"),
            st(c, "subject"),
            st(g(c, "changed"), "kind"),
        );
        items.push([
            st(ch, "id").to_owned(),
            st(ch, "relation").to_owned(),
            cat.to_owned(),
            subj.to_owned(),
            kind.to_owned(),
        ]);
        if cat == "system" || subj.is_empty() || kind == "removed" {
            continue;
        }
        let mark = if kind == "added" { "added" } else { "changed" };
        let key = (cat.to_owned(), subj.to_owned());
        if marks.get(&key).map(String::as_str) != Some("added") {
            marks.insert(key, mark.to_owned());
        }
    }
    (marks, items)
}

/// The Changes section.
pub fn render_changes(
    m: &Model<'_>,
    diff: Option<&Value>,
    diff_ref: Option<&str>,
    items: &[[String; 5]],
) -> String {
    if diff.is_none() {
        return r#"<p class="dim">No semantic diff was passed to the generator (<code>--diff</code>).</p>"#.into();
    }
    let head = format!(
        "since <code>{}</code>",
        esc(diff_ref.unwrap_or("the previous revision"))
    );
    if items.is_empty() {
        return format!(r#"<p class="lede">No semantic change {head}.</p>"#);
    }
    let rows: String = items
        .iter()
        .map(|[_, rel, cat, subj, kind]| {
            let name = if m.chg.contains_key(&(cat.clone(), subj.clone())) {
                let a = m.a(cat, subj);
                format!(r##"<a class="ref mono" href="#{}" data-ref="{}">{}</a>"##, esc(&a), esc(&a), esc(subj))
            } else {
                format!(r#"<span class="mono">{}</span>"#, esc(subj))
            };
            format!(r#"<tr><td class="mono">{}</td><td>{name}</td><td class="mono">{}</td><td class="mono">{}</td></tr>"#, esc(cat), esc(kind), esc(rel))
        })
        .collect();
    format!(
        r#"<p class="lede">{} semantic change(s) {head}, from <code>ess verify diff</code>. Added and changed declarations carry a badge on the diagrams and cards.</p><table class="srcidx"><tr><th>category</th><th>declaration</th><th>change</th><th>relation</th></tr>{rows}</table>"#,
        items.len()
    )
}

/// The label of a run's origin.
pub fn origin_label(origin: &str) -> &'static str {
    match origin {
        "authored" => "authored",
        "synthesized" => "synthesized",
        _ => "seeded run",
    }
}

/// The run picker: a button naming the current run, and the popover the scripts fill.
fn run_picker(sim: &Value) -> String {
    let demo = st(sim, "demo");
    let cur = arr(sim, "traces").iter().find(|t| st(t, "id") == demo);
    let (name, origin, n) = cur.map_or(("no runs", "seeded", 0), |t| {
        (st(t, "title"), st(t, "origin"), arr(t, "steps").len())
    });
    format!(
        r#"<div class="rp" id="dm-run"><button type="button" class="rp-btn" id="dm-trace" aria-haspopup="listbox" aria-expanded="false" aria-controls="dm-run-pop" title="Choose a scenario or run (r)"><span class="rp-eyebrow">run</span><span class="rp-name">{}</span><span class="rp-kind k-{origin}">{}</span><span class="rp-steps">{n} steps</span><span class="rp-caret" aria-hidden="true"></span></button><div class="rp-scrim" hidden></div><div class="rp-pop" id="dm-run-pop" role="dialog" aria-label="Choose a run" hidden><div class="rp-head"><input id="dm-run-q" type="text" role="combobox" aria-autocomplete="list" aria-expanded="true" aria-controls="dm-run-list" autocomplete="off" spellcheck="false" placeholder="Search scenarios and runs" aria-label="Search scenarios and runs"><span class="rp-count" aria-live="polite"></span></div><div id="dm-run-list" class="rp-list" role="listbox" aria-label="Scenarios and runs"></div><div class="rp-foot"><kbd>↑</kbd><kbd>↓</kbd> move · <kbd>Enter</kbd> play · <kbd>Esc</kbd> close</div></div></div>"#,
        esc(name),
        origin_label(origin)
    )
}
