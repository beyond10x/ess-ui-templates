//! SVG and HTML fragments of the Model tab. Every name drawn comes from the model built from the IR.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write as _;

use serde_json::{json, Value};

use crate::model::{
    arr, assign_text, condition_text, display, g, obj, predicate_text, predicate_variants, short,
    shortest_paths, st, strs, summary, text, type_label, Cause, Edge, Model, Unrendered,
    GUARD_KINDS, REFUSAL_KINDS,
};

const CHAR_W: f64 = 6.3;

/// HTML-escapes text, quotes included.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#x27;"),
            c => o.push(c),
        }
    }
    o
}

/// An element-id-safe form of a name.
pub fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The width text is drawn at.
#[allow(clippy::cast_precision_loss)]
pub fn text_w(s: &str, cw: f64) -> f64 {
    s.chars().count() as f64 * cw
}

/// A number as a coordinate: integral values without a fraction.
pub fn n(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{x:.0}")
    } else {
        format!("{x:.1}")
    }
}

fn wrap(t: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in t.split_whitespace() {
        if !cur.is_empty() && cur.chars().count() + 1 + w.chars().count() > width {
            lines.push(std::mem::take(&mut cur));
            cur = w.to_owned();
        } else if cur.is_empty() {
            cur = w.to_owned();
        } else {
            cur.push(' ');
            cur.push_str(w);
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The domain(s) a declaration belongs to, for domain focus.
pub fn dom_of(m: &Model<'_>, kind: &str, ident: &str) -> String {
    match kind {
        "component" => m
            .comps
            .get(ident)
            .map(|c| strs(c, "owns").join(" "))
            .unwrap_or_default(),
        "domain" => ident.to_owned(),
        _ => {
            let mut best = "";
            for d in m.domains.keys() {
                if ident.starts_with(&format!("{d}.")) && d.len() > best.len() {
                    best = d;
                }
            }
            best.to_owned()
        }
    }
}

/// The data attributes that tag one declaration on the page.
pub fn tag(m: &Model<'_>, kind: &str, ident: &str, primary: bool, rel: &[String]) -> String {
    let a = m.a(kind, ident);
    let mut s = format!(
        r#"data-ess-kind="{}" data-ess-id="{}" data-ess-a="{}""#,
        esc(kind),
        esc(ident),
        esc(&a)
    );
    let dom = dom_of(m, kind, ident);
    if !dom.is_empty() {
        let _ = write!(s, r#" data-dom="{}""#, esc(&dom));
    }
    if let Some(c) = m.chg.get(&(kind.to_owned(), ident.to_owned())) {
        let _ = write!(s, r#" data-chg="{c}""#);
    }
    if primary {
        if let Some(p) = m.anchors.borrow_mut().place(kind, ident) {
            let _ = write!(s, r#" id="{}""#, esc(&p));
        }
    }
    let rel: BTreeSet<&String> = rel.iter().filter(|r| !r.is_empty() && **r != a).collect();
    if !rel.is_empty() {
        let _ = write!(
            s,
            r#" data-ess-rel="{}""#,
            esc(&rel.into_iter().cloned().collect::<Vec<_>>().join(" "))
        );
    }
    s
}

/// The source link of a declaration, or "".
pub fn src_link(m: &Model<'_>, kind: &str, ident: &str) -> String {
    let Some((f, line)) = m.src.get(&(kind.to_owned(), ident.to_owned())) else {
        return String::new();
    };
    format!(
        r#"<a class="src" href="{}" data-src-kind="{}" data-src-id="{}" data-src-file="{}" data-src-line="{line}" title="{}:{line}">{}:{line}</a>"#,
        esc(&(m.link)(f, *line)),
        esc(kind),
        esc(ident),
        esc(f),
        esc(f),
        esc(f)
    )
}

fn card_head(m: &Model<'_>, kind: &str, ident: &str, title: Option<&str>) -> String {
    let a = m.a(kind, ident);
    let badge = m
        .chg
        .get(&(kind.to_owned(), ident.to_owned()))
        .map(|c| format!(r#" <span class="chgbadge {c}">{c}</span>"#))
        .unwrap_or_default();
    format!(
        r##"<div class="card-h"><span class="nm">{}{badge}</span><span class="sub">{}</span><span class="card-tools">{}<button class="copy" data-copy="{}" title="Copy link to this declaration" aria-label="Copy link">#</button></span></div>"##,
        esc(title.unwrap_or_else(|| short(ident))),
        esc(ident),
        src_link(m, kind, ident),
        esc(&a)
    )
}

/// Where the segment from a rectangle's centre towards a point leaves the rectangle.
pub fn clip_rect(cx: f64, cy: f64, w: f64, h: f64, tx: f64, ty: f64) -> (f64, f64) {
    let (dx, dy) = (tx - cx, ty - cy);
    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }
    let sx = if dx == 0.0 {
        f64::INFINITY
    } else {
        (w / 2.0) / dx.abs()
    };
    let sy = if dy == 0.0 {
        f64::INFINITY
    } else {
        (h / 2.0) / dy.abs()
    };
    let s = sx.min(sy);
    (cx + dx * s, cy + dy * s)
}

/// The colour a domain is drawn in, as a CSS custom property.
pub fn dom_style(m: &Model<'_>, d: &str) -> String {
    format!(
        "--dc:{}",
        m.dom_colour.get(d).map_or("#888", String::as_str)
    )
}

// ---- lifecycles ---------------------------------------------------------------------------------

/// The anchors a cause relates to.
pub fn cause_anchors(m: &Model<'_>, c: &Cause) -> Vec<String> {
    let mut out = vec![
        m.a("command", &c.command),
        m.a("outcome", &format!("{}/{}", c.command, c.outcome)),
    ];
    out.extend(c.actors.iter().map(|a| m.a("actor", a)));
    out.extend(c.components.iter().map(|x| m.a("component", x)));
    out.extend(c.emits.iter().map(|e| m.a("event", st(e, "event"))));
    out
}

fn label_lines(m: &Model<'_>, entity: &str, tname: &str) -> Vec<(&'static str, String)> {
    let mut lines = vec![("t-name", tname.to_owned())];
    let empty = Vec::new();
    let cs = m
        .causes
        .get(&(entity.to_owned(), tname.to_owned()))
        .unwrap_or(&empty);
    if cs.is_empty() {
        lines.push(("t-none", "no command outcome in IR".into()));
    }
    for c in cs {
        lines.push(("t-cmd", format!("{} › {}", short(&c.command), c.outcome)));
        let act = c
            .actors
            .iter()
            .map(|a| short(a))
            .collect::<Vec<_>>()
            .join(", ");
        let act = if act.is_empty() {
            "no actor".to_owned()
        } else {
            act
        };
        let guarded = c.kind == "guarded" || c.kind == "default";
        lines.push(if guarded {
            ("t-guard", format!("[{}] · {act}", c.condition))
        } else {
            ("t-actor", act)
        });
        for gd in &c.refused_if {
            let err = gd
                .get("error")
                .and_then(Value::as_str)
                .map(|e| format!(" → {}", short(e)))
                .unwrap_or_default();
            lines.push((
                "t-refuse",
                format!("refused if {}{err}", st(gd, "condition")),
            ));
        }
    }
    lines
}

fn edge_kind(m: &Model<'_>, entity: &str, tname: &str) -> &'static str {
    let Some(cs) = m
        .causes
        .get(&(entity.to_owned(), tname.to_owned()))
        .filter(|c| !c.is_empty())
    else {
        return "none";
    };
    for k in ["guarded", "default", "unconditional"] {
        if cs.iter().any(|c| c.kind == k) {
            return k;
        }
    }
    "other"
}

fn entry_lines(cr: &[&Cause]) -> Vec<(&'static str, String)> {
    if cr.is_empty() {
        return vec![("t-none", "no creating command in IR".into())];
    }
    let mut lines = Vec::new();
    for c in cr {
        lines.push(("t-cmd", format!("{} › {}", short(&c.command), c.outcome)));
        let act = c
            .actors
            .iter()
            .map(|a| short(a))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push((
            "t-actor",
            if act.is_empty() {
                "no actor".into()
            } else {
                act
            },
        ));
    }
    lines
}

#[derive(Clone, Copy, PartialEq)]
enum Anchor {
    Start,
    Middle,
    End,
}

impl Anchor {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Middle => "middle",
            Self::End => "end",
        }
    }
}

type Lab = (f64, f64, Anchor);

fn label_box(lab: Lab, lines: &[(&str, String)]) -> (f64, f64, f64, f64) {
    let (x, y, anc) = lab;
    let w = lines
        .iter()
        .map(|(_, t)| text_w(t, CHAR_W))
        .fold(0.0, f64::max)
        + 12.0;
    #[allow(clippy::cast_precision_loss)]
    let h = 13.0 * lines.len() as f64 + 6.0;
    let mut rx = match anc {
        Anchor::End => x - w,
        Anchor::Middle => x - w / 2.0,
        Anchor::Start => x,
    };
    rx += if anc == Anchor::End { 6.0 } else { 0.0 } - if anc == Anchor::Start { 6.0 } else { 0.0 };
    (rx, y - h / 2.0, rx + w, y + h / 2.0)
}

fn svg_label(lab: Lab, lines: &[(&str, String)], kind: &str, r: &str, attrs: &str) -> String {
    let (x, _, anc) = lab;
    let (rx, top, x1, y1) = label_box(lab, lines);
    let mut out = format!(
        r#"<g class="elabel k-{kind}" data-edge="{r}" {attrs}><rect x="{rx:.1}" y="{top:.1}" width="{:.1}" height="{:.1}" rx="5"/>"#,
        x1 - rx,
        y1 - top
    );
    for (i, (cls, t)) in lines.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let ty = top + 14.0 + i as f64 * 13.0;
        let _ = write!(
            out,
            r#"<text class="{cls}" x="{x:.1}" y="{ty:.1}" text-anchor="{}">{}</text>"#,
            anc.as_str(),
            esc(t)
        );
    }
    out.push_str("</g>");
    out
}

/// Ranks and columns of a lifecycle laid out from its initial state; unreachable states apart.
pub fn ranks(
    states: &[String],
    init: &str,
    term: &BTreeSet<String>,
    edges: &[(String, String, String)],
) -> (BTreeMap<String, i64>, BTreeMap<String, i64>, Vec<String>) {
    let mut rank: BTreeMap<String, i64> = BTreeMap::new();
    let mut parent: BTreeMap<String, String> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    if states.iter().any(|s| s == init) {
        rank.insert(init.to_owned(), 0);
        order.push(init.to_owned());
        let mut q = VecDeque::from([init.to_owned()]);
        while let Some(s) = q.pop_front() {
            for (f, t, _) in edges {
                if *f == s && !rank.contains_key(t) {
                    rank.insert(t.clone(), rank[&s] + 1);
                    parent.insert(t.clone(), s.clone());
                    order.push(t.clone());
                    q.push_back(t.clone());
                }
            }
        }
    }
    let idx = |s: &String| states.iter().position(|x| x == s).unwrap_or(0);
    let mut col: BTreeMap<String, i64> = BTreeMap::new();
    if !rank.is_empty() {
        let mut ends: Vec<&String> = states
            .iter()
            .filter(|s| term.contains(*s) && rank.contains_key(*s))
            .collect();
        if ends.is_empty() {
            ends = order.iter().collect();
        }
        let end = ends
            .iter()
            .max_by_key(|s| (rank[**s], -(i64::try_from(idx(s)).unwrap_or(0))))
            .copied()
            .cloned()
            .unwrap_or_default();
        let mut s = Some(end);
        while let Some(x) = s {
            col.insert(x.clone(), 0);
            s = parent.get(&x).cloned();
        }
        let mut occupied: BTreeSet<(i64, i64)> = col.keys().map(|s| (rank[s], 0)).collect();
        let mut rest: Vec<&String> = rank.keys().filter(|s| !col.contains_key(*s)).collect();
        rest.sort_by_key(|s| (rank[*s], idx(s)));
        for s in rest {
            let pc = parent.get(s).and_then(|p| col.get(p)).copied().unwrap_or(0);
            'outer: for d in 0..20 {
                let cands = if d == 0 {
                    vec![pc]
                } else {
                    vec![pc + d, pc - d]
                };
                for c in cands {
                    if !occupied.contains(&(rank[s], c)) {
                        col.insert(s.clone(), c);
                        occupied.insert((rank[s], c));
                        break 'outer;
                    }
                }
            }
        }
    }
    let maxc = col.values().copied().max().unwrap_or(0);
    let mut apart = Vec::new();
    for (i, s) in states
        .iter()
        .filter(|s| !col.contains_key(*s))
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .enumerate()
    {
        col.insert(s.clone(), maxc + 1);
        rank.insert(s.clone(), i64::try_from(i).unwrap_or(0));
        apart.push(s);
    }
    (rank, col, apart)
}

struct OutEdge {
    id: String,
    from: String,
    to: String,
    transition: String,
    d: String,
    kind: &'static str,
    pts: Vec<(f64, f64)>,
    lines: Vec<(&'static str, String)>,
    lab: Lab,
    vert: Option<(f64, f64)>,
    causes: Vec<Cause>,
    tid: String,
}

/// One lifecycle diagram and the data its player needs.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn render_lifecycle(
    m: &Model<'_>,
    ename: &str,
    svg_id: &str,
    unr: &mut Unrendered,
    headline: bool,
) -> (String, Value) {
    let ent = &m.entities[ename];
    let lc = g(ent, "lifecycle");
    let states = strs(lc, "states");
    let init = st(lc, "initial").to_owned();
    let term: BTreeSet<String> = strs(lc, "terminal").into_iter().collect();
    let mut edges: Vec<(String, String, String)> = Vec::new();
    for t in arr(lc, "transitions") {
        for f in strs(t, "from") {
            if states.contains(&f) && states.iter().any(|s| s == st(t, "to")) {
                edges.push((f, st(t, "to").to_owned(), st(t, "name").to_owned()));
            } else {
                unr.add(format!("entity {ename}: transition {}: {f} → {} names a state not in the lifecycle's states; not drawn", st(t, "name"), st(t, "to")));
            }
        }
    }
    if !states.contains(&init) {
        unr.add(format!(
            "entity {ename}: initial state `{init}` is not among its states"
        ));
    }
    let (rank, col, apart) = ranks(&states, &init, &term, &edges);
    for s in &apart {
        unr.add(format!(
            "entity {ename}: state `{s}` is not reachable from the initial state; drawn apart"
        ));
    }
    let (colg, rankg, nh) = (330.0, 104.0, 46.0);
    let nodew: BTreeMap<&String, f64> = states
        .iter()
        .map(|s| (s, (text_w(s, 8.2) + 34.0).max(128.0)))
        .collect();
    let pos: BTreeMap<&String, (f64, f64)> = states
        .iter()
        .map(|s| (s, (col[s] as f64 * colg, 70.0 + rank[s] as f64 * rankg)))
        .collect();
    let pairs: BTreeSet<(&String, &String)> = edges.iter().map(|(a, b, _)| (a, b)).collect();
    let mincol = col.values().copied().min().unwrap_or(0) as f64;
    let maxcol = col.values().copied().max().unwrap_or(0) as f64;
    let maxw = nodew.values().copied().fold(120.0_f64, f64::max);
    let right_ch = maxcol * colg + maxw / 2.0 + 40.0;
    let left_ch = mincol * colg - maxw / 2.0 - 40.0;
    let mut lanes: BTreeMap<&str, Vec<(f64, f64, i64)>> =
        BTreeMap::from([("R", Vec::new()), ("L", Vec::new())]);
    let mut out_edges: Vec<OutEdge> = Vec::new();
    let mut same_pair: BTreeMap<(String, String), i64> = BTreeMap::new();
    let mut labelled: BTreeSet<String> = BTreeSet::new();
    for (a, b, name) in &edges {
        let ((ax, ay), (bx, by)) = (pos[a], pos[b]);
        let (dr, dc) = (rank[b] - rank[a], col[b] - col[a]);
        let lid = format!("{svg_id}-t-{}-{}-{}", slug(name), slug(a), slug(b));
        let lines = if labelled.contains(name) {
            vec![("t-name", name.clone())]
        } else {
            labelled.insert(name.clone());
            label_lines(m, ename, name)
        };
        let kind = edge_kind(m, ename, name);
        let k = *same_pair.get(&(a.clone(), b.clone())).unwrap_or(&0);
        same_pair.insert((a.clone(), b.clone()), k + 1);
        let mut pts = Vec::new();
        let d;
        let mut lab: Lab = (0.0, 0.0, Anchor::Middle);
        let mut vert = None;
        if a == b {
            let (x1, y1) = (ax + nodew[a] / 2.0, ay - 8.0);
            d = format!(
                "M{x1:.1},{y1:.1} C{:.1},{:.1} {:.1},{:.1} {x1:.1},{:.1}",
                x1 + 60.0,
                y1 - 40.0,
                x1 + 60.0,
                y1 + 56.0,
                y1 + 16.0
            );
            lab = (x1 + 64.0, ay, Anchor::Start);
            pts.push((x1 + 40.0, y1 + 8.0));
        } else if dr.abs() <= 1 && !(dr == 0 && dc.abs() > 1) {
            let (sx, sy) = clip_rect(ax, ay, nodew[a] + 6.0, nh + 6.0, bx, by);
            let (tx, ty) = clip_rect(bx, by, nodew[b] + 6.0, nh + 6.0, ax, ay);
            let bend = if pairs.contains(&(b, a)) || k > 0 {
                26.0 * (k + 1) as f64
            } else {
                0.0
            };
            let (mx, my) = ((sx + tx) / 2.0, (sy + ty) / 2.0);
            let l = ((tx - sx).powi(2) + (ty - sy).powi(2)).sqrt().max(1.0);
            let (px, py) = (-(ty - sy) / l, (tx - sx) / l);
            let (cx, cy) = (mx + px * bend, my + py * bend);
            d = format!("M{sx:.1},{sy:.1} Q{cx:.1},{cy:.1} {tx:.1},{ty:.1}");
            let (qx, qy) = (
                0.25 * sx + 0.5 * cx + 0.25 * tx,
                0.25 * sy + 0.5 * cy + 0.25 * ty,
            );
            #[allow(clippy::cast_possible_truncation)]
            let nseg = ((l / 14.0) as i64).max(4);
            for i in 1..nseg {
                let t = i as f64 / nseg as f64;
                pts.push((
                    (1.0 - t).powi(2) * sx + 2.0 * (1.0 - t) * t * cx + t * t * tx,
                    (1.0 - t).powi(2) * sy + 2.0 * (1.0 - t) * t * cy + t * t * ty,
                ));
            }
            if dc == 0 && dr != 0 && bend == 0.0 {
                vert = Some((qx, qy));
            } else if bend != 0.0 {
                let right = px * bend > 0.0;
                lab = (
                    qx + if right { 10.0 } else { -10.0 },
                    qy,
                    if right { Anchor::Start } else { Anchor::End },
                );
            } else {
                lab = (qx, qy, Anchor::Middle);
            }
        } else {
            let side = if col[b] > 0 || (col[b] == 0 && col[a] >= 0) {
                "R"
            } else {
                "L"
            };
            let (lo, hi) = if ay < by { (ay, by) } else { (by, ay) };
            let mut lane = 0;
            while lanes[side]
                .iter()
                .any(|(l0, l1, ln)| *ln == lane && !(hi < l0 - 24.0 || lo > l1 + 24.0))
            {
                lane += 1;
            }
            lanes.get_mut(side).unwrap().push((lo, hi, lane));
            let x = if side == "R" {
                right_ch + lane as f64 * 26.0
            } else {
                left_ch - lane as f64 * 26.0
            };
            let sgn = if side == "R" { 1.0 } else { -1.0 };
            let (sx, sy) = (ax + sgn * nodew[a] / 2.0, ay);
            let (tx, ty) = (bx + sgn * nodew[b] / 2.0, by);
            let dy = if ty > sy { 1.0 } else { -1.0 };
            let r = 14.0;
            d = format!(
                "M{sx:.1},{sy:.1} L{:.1},{sy:.1} Q{x:.1},{sy:.1} {x:.1},{:.1} L{x:.1},{:.1} Q{x:.1},{ty:.1} {:.1},{ty:.1} L{tx:.1},{ty:.1}",
                x - sgn * r,
                sy + dy * r,
                ty - dy * r,
                x - sgn * r
            );
            lab = (
                x + sgn * 10.0,
                (sy + ty) / 2.0,
                if side == "R" {
                    Anchor::Start
                } else {
                    Anchor::End
                },
            );
            for (p0x, p0y, p1x, p1y) in [(sx, sy, x, sy), (x, sy, x, ty), (x, ty, tx, ty)] {
                #[allow(clippy::cast_possible_truncation)]
                let seg = (((p1x - p0x).abs() + (p1y - p0y).abs()) / 14.0) as i64;
                let seg = seg.max(1);
                for i in 1..seg {
                    pts.push((
                        p0x + (p1x - p0x) * i as f64 / seg as f64,
                        p0y + (p1y - p0y) * i as f64 / seg as f64,
                    ));
                }
            }
        }
        let causes = m
            .causes
            .get(&(ename.to_owned(), name.clone()))
            .cloned()
            .unwrap_or_default();
        out_edges.push(OutEdge {
            id: lid,
            from: a.clone(),
            to: b.clone(),
            transition: name.clone(),
            d,
            kind,
            pts,
            lines,
            lab,
            vert,
            causes,
            tid: format!("{ename}#{name}:{a}->{b}"),
        });
    }
    let snapshot: Vec<(String, String)> = out_edges
        .iter()
        .map(|o| (o.from.clone(), o.to.clone()))
        .collect();
    for (i, oe) in out_edges.iter_mut().enumerate() {
        let Some((qx, qy)) = oe.vert else { continue };
        let c = col[&oe.from];
        let rs: BTreeSet<i64> = [rank[&oe.from], rank[&oe.to]].into_iter().collect();
        let score = |sg: i64| -> usize {
            let mut n = states
                .iter()
                .filter(|s| col[*s] == c + sg && rs.contains(&rank[*s]))
                .count();
            n += snapshot
                .iter()
                .enumerate()
                .filter(|(j, (f, t))| {
                    *j != i
                        && rs.contains(&rank[f])
                        && rs.contains(&rank[t])
                        && (col[f] == c + sg || col[t] == c + sg)
                })
                .count();
            n
        };
        let sg = if score(-1) <= score(1) { -1.0 } else { 1.0 };
        oe.lab = (
            qx + sg * 12.0,
            qy,
            if sg < 0.0 { Anchor::End } else { Anchor::Start },
        );
    }
    let creators = m.creators.get(ename).cloned().unwrap_or_default();
    let cr: Vec<&Cause> = creators
        .iter()
        .filter(|c| c.into.as_deref().unwrap_or(&init) == init)
        .collect();
    let entry = pos.get(&init).map(|(ix, iy)| {
        (
            format!("{svg_id}-entry"),
            format!(
                "M{ix:.1},{:.1} L{ix:.1},{:.1}",
                iy - 64.0,
                iy - nh / 2.0 - 4.0
            ),
        )
    });
    let mut obstacles: Vec<(f64, f64, f64, f64)> = states
        .iter()
        .map(|s| {
            (
                pos[s].0 - nodew[s] / 2.0 - 6.0,
                pos[s].1 - nh / 2.0 - 6.0,
                pos[s].0 + nodew[s] / 2.0 + 6.0,
                pos[s].1 + nh / 2.0 + 6.0,
            )
        })
        .collect();
    if entry.is_some() {
        let (ix, iy) = pos[&init];
        obstacles.push(label_box(
            (ix + 12.0, iy - 52.0, Anchor::Start),
            &entry_lines(&cr),
        ));
    }
    let mut offsets: Vec<(i64, i64)> = Vec::new();
    for dx in (-220..=220).step_by(10) {
        for dy in (-110..=110).step_by(8) {
            offsets.push((dx, dy));
        }
    }
    offsets.sort_by(|a, b| {
        let ka = a.0.abs() as f64 * 0.7 + a.1.abs() as f64;
        let kb = b.0.abs() as f64 * 0.7 + b.1.abs() as f64;
        ka.partial_cmp(&kb)
            .unwrap()
            .then(a.0.cmp(&b.0))
            .then(a.1.cmp(&b.1))
    });
    let edge_pts: Vec<(f64, f64)> = out_edges.iter().flat_map(|o| o.pts.clone()).collect();
    let mut order: Vec<usize> = (0..out_edges.len()).collect();
    order.sort_by_key(|&i| {
        (
            i32::from(out_edges[i].lab.2 == Anchor::Middle),
            -(i64::try_from(out_edges[i].lines.len()).unwrap_or(0)),
        )
    });
    for i in order {
        let (x, y, anc) = out_edges[i].lab;
        let mut best: Option<(f64, f64)> = None;
        let mut best_score: Option<f64> = None;
        for (dx, dy) in &offsets {
            let dist = dx.abs() as f64 * 0.7 + dy.abs() as f64;
            if best_score.is_some_and(|b| dist >= b) {
                break;
            }
            let b = label_box((x + *dx as f64, y + *dy as f64, anc), &out_edges[i].lines);
            if obstacles
                .iter()
                .any(|o| b.0 < o.2 && o.0 < b.2 && b.1 < o.3 && o.1 < b.3)
            {
                continue;
            }
            let crossings = edge_pts
                .iter()
                .filter(|(px, py)| {
                    b.0 - 2.0 < *px && *px < b.2 + 2.0 && b.1 - 2.0 < *py && *py < b.3 + 2.0
                })
                .count();
            let score = dist + 60.0 * crossings as f64;
            if best_score.is_none_or(|bs| score < bs) {
                best = Some((*dx as f64, *dy as f64));
                best_score = Some(score);
            }
        }
        if let Some((bx, by)) = best {
            out_edges[i].lab = (x + bx, y + by, anc);
        }
        obstacles.push(label_box(out_edges[i].lab, &out_edges[i].lines));
    }
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for s in &states {
        let (x, y) = pos[s];
        xs.extend([x - nodew[s] / 2.0, x + nodew[s] / 2.0]);
        ys.extend([y - nh / 2.0, y + nh / 2.0]);
    }
    for oe in &out_edges {
        let b = label_box(oe.lab, &oe.lines);
        xs.extend([b.0, b.2]);
        ys.extend([b.1, b.3]);
    }
    for (side, ls) in &lanes {
        for (_, _, lane) in ls {
            xs.push(if *side == "R" {
                right_ch + *lane as f64 * 26.0 + 10.0
            } else {
                left_ch - *lane as f64 * 26.0 - 10.0
            });
        }
    }
    ys.push(-10.0);
    if entry.is_some() {
        let cl = entry_lines(&cr);
        let (ix, _) = pos[&init];
        xs.extend([
            ix + 12.0,
            ix + 12.0
                + cl.iter()
                    .map(|(_, t)| text_w(t, CHAR_W))
                    .fold(0.0, f64::max)
                + 14.0,
        ]);
    }
    let mut upd_lines: Vec<(Cause, String)> = Vec::new();
    for c in &creators {
        let target = c.into.clone().unwrap_or_else(|| init.clone());
        if target != init {
            if !states.contains(&target) {
                unr.add(format!(
                    "entity {ename}: creation target `{target}` is not among its states"
                ));
            }
            upd_lines.push((
                c.clone(),
                format!(
                    "+ {} › {} · creates into {target}",
                    short(&c.command),
                    c.outcome
                ),
            ));
        }
    }
    for c in m.updaters.get(ename).into_iter().flatten() {
        let mut t = format!("Δ {} › {}", short(&c.command), c.outcome);
        if c.kind == "guarded" || c.kind == "default" {
            let _ = write!(t, " [{}]", c.condition);
        }
        if !c.sets.is_empty() {
            let _ = write!(t, " sets {}", c.sets.join(", "));
        }
        t.push_str(" · state unchanged");
        upd_lines.push((c.clone(), t));
    }
    let dels = m.deleters.get(ename).cloned().unwrap_or_default();
    for c in &dels {
        let mut t = format!("✕ {} › {}", short(&c.command), c.outcome);
        if c.kind == "guarded" || c.kind == "default" {
            let _ = write!(t, " [{}]", c.condition);
        }
        t.push_str(" · removes the instance, from any state");
        upd_lines.push((c.clone(), t));
    }
    let minf = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let maxf = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let vx0 = minf(&xs) - 24.0;
    let mut vx1 = maxf(&xs) + 24.0;
    let vy0 = minf(&ys) - 10.0;
    let mut vy1 = maxf(&ys) + 24.0;
    let upd_y = vy1 + 6.0;
    if !upd_lines.is_empty() {
        xs.push(
            vx0 + 24.0
                + upd_lines
                    .iter()
                    .map(|(_, t)| text_w(t, 6.9))
                    .fold(0.0, f64::max)
                + 30.0,
        );
        vx1 = vx1.max(maxf(&xs));
        vy1 = upd_y + 26.0 + 22.0 * upd_lines.len() as f64;
    }
    let ent_a = m.a("entity", ename);
    let mut parts = vec![format!(
        r#"<svg id="{svg_id}" class="lifecycle{}" viewBox="{vx0:.0} {vy0:.0} {:.0} {:.0}" style="max-width:{:.0}px" role="img" aria-label="Lifecycle of {}" data-entity-a="{}">"#,
        if headline { " headline" } else { "" },
        vx1 - vx0,
        vy1 - vy0,
        vx1 - vx0,
        esc(ename),
        esc(&ent_a)
    )];
    parts.push(r#"<g class="edges">"#.into());
    for oe in &out_edges {
        let mut rel = vec![
            m.a("state", &format!("{ename}#{}", oe.from)),
            m.a("state", &format!("{ename}#{}", oe.to)),
        ];
        for c in &oe.causes {
            rel.extend(cause_anchors(m, c));
        }
        parts.push(format!(
            r#"<path class="edge k-{}" data-p="{}" d="{}" marker-end="url(#arrow-{})" {}><title>{}: {} → {}</title></path>"#,
            oe.kind,
            oe.id,
            oe.d,
            oe.kind,
            tag(m, "transition", &oe.tid, true, &rel),
            esc(&oe.transition),
            esc(&oe.from),
            esc(&oe.to)
        ));
    }
    if let Some((eid, ed)) = &entry {
        let (ix, iy) = pos[&init];
        parts.push(format!(
            r#"<circle class="entry-dot" cx="{ix:.1}" cy="{:.1}" r="5"/>"#,
            iy - 68.0
        ));
        parts.push(format!(
            r#"<path class="edge k-entry" data-p="{eid}" d="{ed}" marker-end="url(#arrow-entry)"/>"#
        ));
    }
    parts.push(r#"</g><g class="labels">"#.into());
    for oe in &out_edges {
        let mut rel: Vec<String> = Vec::new();
        for c in &oe.causes {
            rel.extend(cause_anchors(m, c));
        }
        rel.push(m.a("transition", &oe.tid));
        let rel: BTreeSet<String> = rel.into_iter().collect();
        parts.push(svg_label(
            oe.lab,
            &oe.lines,
            oe.kind,
            &oe.id,
            &format!(
                r#"data-ess-rel="{}""#,
                esc(&rel.into_iter().collect::<Vec<_>>().join(" "))
            ),
        ));
    }
    if let Some((eid, _)) = &entry {
        let (ix, iy) = pos[&init];
        let rel: BTreeSet<String> = cr.iter().flat_map(|c| cause_anchors(m, c)).collect();
        let attrs = if rel.is_empty() {
            String::new()
        } else {
            format!(
                r#"data-ess-rel="{}""#,
                esc(&rel.into_iter().collect::<Vec<_>>().join(" "))
            )
        };
        parts.push(svg_label(
            (ix + 12.0, iy - 52.0, Anchor::Start),
            &entry_lines(&cr),
            "entry",
            eid,
            &attrs,
        ));
    }
    parts.push(r#"</g><g class="nodes">"#.into());
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
        parts.push(format!(
            r#"<g class="{}" data-s="{}" data-cx="{x:.1}" data-cy="{y:.1}" {}>"#,
            cls.join(" "),
            esc(s),
            tag(
                m,
                "state",
                &format!("{ename}#{s}"),
                true,
                std::slice::from_ref(&ent_a)
            )
        ));
        if term.contains(s) {
            parts.push(format!(
                r#"<rect class="ring" x="{:.1}" y="{:.1}" width="{:.1}" height="{}" rx="{}"/>"#,
                x - w / 2.0 - 5.0,
                y - nh / 2.0 - 5.0,
                w + 10.0,
                n(nh + 10.0),
                n(nh / 2.0 + 5.0)
            ));
        }
        parts.push(format!(
            r#"<rect class="body" x="{:.1}" y="{:.1}" width="{w:.1}" height="{}" rx="{}"/>"#,
            x - w / 2.0,
            y - nh / 2.0,
            n(nh),
            n(nh / 2.0)
        ));
        let tg: Vec<&str> = [("initial", *s == init), ("terminal", term.contains(s))]
            .iter()
            .filter(|(_, on)| *on)
            .map(|(t, _)| *t)
            .collect();
        let tg = tg.join(" · ");
        parts.push(format!(
            r#"<text x="{x:.1}" y="{:.1}" text-anchor="middle">{}</text>"#,
            y + if tg.is_empty() { 4.5 } else { 0.0 },
            esc(s)
        ));
        if !tg.is_empty() {
            parts.push(format!(
                r#"<text class="tag" x="{x:.1}" y="{:.1}" text-anchor="middle">{tg}</text>"#,
                y + 13.0
            ));
        }
        parts.push("</g>".into());
    }
    parts.push("</g>".into());
    if !upd_lines.is_empty() {
        let head = if dels.is_empty() {
            "field changes that move no state (updates)"
        } else {
            "field changes that move no state, and removals (updates, deletes)"
        };
        parts.push(format!(
            r#"<g class="updates"><text class="upd-h" x="{:.1}" y="{:.1}">{head}</text>"#,
            vx0 + 24.0,
            upd_y + 14.0
        ));
        for (i, (c, t)) in upd_lines.iter().enumerate() {
            let yy = upd_y + 36.0 + 22.0 * i as f64;
            parts.push(format!(
                r#"<g class="upd" {}><rect x="{:.1}" y="{:.1}" width="{:.1}" height="20" rx="6"/><text x="{:.1}" y="{yy:.1}">{}</text></g>"#,
                tag(m, "outcome", &format!("{}/{}", c.command, c.outcome), false, &cause_anchors(m, c)),
                vx0 + 18.0,
                yy - 14.0,
                text_w(t, 6.9) + 24.0,
                vx0 + 30.0,
                esc(t)
            ));
        }
        parts.push("</g>".into());
    }
    let (tx, ty) = pos.get(&init).copied().unwrap_or((0.0, 0.0));
    parts.push(format!(
        r#"<circle class="token" r="8" cx="{tx:.1}" cy="{ty:.1}"/>"#
    ));
    parts.push("</svg>".into());
    let js_edges: Vec<Value> = out_edges
        .iter()
        .map(|oe| json!({"id": oe.id, "from": oe.from, "to": oe.to, "transition": oe.transition, "a": m.a("transition", &oe.tid), "causes": oe.causes}))
        .collect();
    let path_edges: Vec<Edge> = out_edges
        .iter()
        .map(|o| Edge {
            id: o.id.clone(),
            from: o.from.clone(),
            to: o.to.clone(),
        })
        .collect();
    let term_list: Vec<String> = states
        .iter()
        .filter(|s| term.contains(*s))
        .cloned()
        .collect();
    let paths = if states.contains(&init) {
        shortest_paths(&init, &term_list, &path_edges)
    } else {
        Vec::new()
    };
    let js = json!({"svg": svg_id, "entity": ename, "initial": init, "terminal": term.iter().collect::<Vec<_>>(), "states": states,
        "edges": js_edges, "paths": paths, "entry": entry.map(|(id, _)| json!({"id": id, "causes": cr}))});
    (parts.concat(), js)
}

// ---- architecture -------------------------------------------------------------------------------

/// Actor colours, in declaration order.
pub const ACTOR_COLOURS: [&str; 6] = [
    "#1f8fb3", "#c27a1a", "#7d63d9", "#23926a", "#c94a6d", "#3f7acc",
];

/// The architecture diagram, and each actor's colour.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn render_architecture(m: &Model<'_>) -> (String, BTreeMap<String, String>) {
    let (w, x0, x1) = (1240.0, 40.0, 1090.0);
    let actors: Vec<&String> = m.actors.keys().collect();
    let colour: BTreeMap<String, String> = actors
        .iter()
        .enumerate()
        .map(|(i, a)| {
            (
                (*a).clone(),
                ACTOR_COLOURS[i % ACTOR_COLOURS.len()].to_owned(),
            )
        })
        .collect();
    let mut parts: Vec<String> = Vec::new();
    let mut y = 20.0;
    let band_h = if actors.is_empty() { 60.0 } else { 118.0 };
    parts.push(format!(
        r#"<g class="band band-actors"><rect x="{}" y="{}" width="{}" height="{}" rx="14"/><text class="band-title" x="{}" y="{}">actors</text>{}</g>"#,
        n(x0), n(y), n(x1 - x0), n(band_h), n(x0 + 18.0), n(y + 26.0),
        if actors.is_empty() { format!(r#"<text class="band-sum" x="{}" y="{}">The IR declares no actors.</text>"#, n(x0 + 18.0), n(y + 46.0)) } else { String::new() }
    ));
    let na = actors.len().max(1) as f64;
    let aw = ((x1 - x0 - 40.0 - 20.0 * (actors.len().max(1) - 1) as f64) / na).min(300.0);
    let mut apos: BTreeMap<&String, (f64, f64)> = BTreeMap::new();
    for (i, a) in actors.iter().enumerate() {
        let ax = x0 + 20.0 + i as f64 * (aw + 20.0);
        let ay = y + 40.0;
        let ao = &m.actors[*a];
        apos.insert(a, (ax + aw / 2.0, ay + 62.0));
        let disp = display(ao);
        let nm = short(a);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let maxc = ((aw - 24.0) / 6.9).max(2.0) as usize;
        let nm_t = if nm.chars().count() <= maxc {
            nm.to_owned()
        } else {
            format!("{}…", nm.chars().take(maxc - 1).collect::<String>())
        };
        let full_t = if a.chars().count() <= maxc + 4 {
            (*a).clone()
        } else {
            format!(
                "…{}",
                a.chars()
                    .rev()
                    .take(maxc + 3)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<String>()
            )
        };
        parts.push(format!(
            r#"<g class="actor" style="--c:{}" {}><rect x="{ax:.1}" y="{}" width="{aw:.1}" height="62" rx="10"/><text class="nm" x="{:.1}" y="{}">{}</text><text class="sub" x="{:.1}" y="{}">{}</text><text class="sub" x="{:.1}" y="{}">may send {}</text><title>{}{}</title></g>"#,
            colour[*a], tag(m, "actor", a, true, &[]), n(ay), ax + 12.0, n(ay + 22.0), esc(&nm_t), ax + 12.0, n(ay + 38.0), esc(&full_t), ax + 12.0, n(ay + 53.0),
            arr(ao, "may").len(), esc(a), disp.map(|d| format!(" — {}", esc(&d))).unwrap_or_default()
        ));
    }
    y += band_h;
    let mut rows: Vec<(Option<String>, Value)> = m
        .comp_order
        .iter()
        .map(|c| (Some(c.clone()), m.comps[c].clone()))
        .collect();
    let unowned: Vec<&String> = m
        .domains
        .keys()
        .filter(|d| !m.dom_owner.contains_key(*d))
        .collect();
    if !unowned.is_empty() {
        rows.push((None, json!({"owns": unowned})));
    }
    let mut cpos: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut bands: Vec<(f64, f64)> = Vec::new();
    for (cname, comp) in &rows {
        y += 46.0;
        let owns = strs(comp, "owns");
        let summ = wrap(&summary(comp), 150);
        let nown = owns.len().max(1) as f64;
        let cw = ((x1 - x0 - 40.0 - 16.0 * (nown - 1.0)) / nown).min(420.0);
        let mut chip_h: f64 = 0.0;
        for d in &owns {
            let ents = m
                .domains
                .get(d)
                .map(|dm| arr(dm, "entities").len())
                .unwrap_or(0);
            chip_h = chip_h.max(58.0 + 17.0 * ents as f64 + 18.0);
        }
        let top_h = 38.0 + 15.0 * summ.len() as f64;
        let bh = top_h + chip_h + 34.0;
        let klass = if cname.is_some() {
            "band"
        } else {
            "band band-unowned"
        };
        let ident = cname.as_ref().map_or_else(
            || r#"data-kind="unowned-domains""#.to_owned(),
            |c| tag(m, "component", c, true, &[]),
        );
        parts.push(format!(
            r#"<g class="{klass}" {ident}><rect x="{}" y="{}" width="{}" height="{}" rx="14"/><text class="band-title" x="{}" y="{}">{}</text>"#,
            n(x0), n(y), n(x1 - x0), n(bh), n(x0 + 18.0), n(y + 26.0),
            cname.as_ref().map_or_else(|| "domains owned by no component in the IR".to_owned(), |c| esc(c))
        ));
        if cname.is_some() {
            let reached = comp
                .get("reached_by")
                .and_then(Value::as_str)
                .map(|r| format!(" · reached_by {r}"))
                .unwrap_or_default();
            let meta = format!(
                "accepts {} · publishes {}{reached}",
                arr(comp, "accepts").len(),
                arr(comp, "publishes").len()
            );
            parts.push(format!(
                r#"<text class="band-meta" x="{}" y="{}" text-anchor="end">{}</text>"#,
                n(x1 - 18.0),
                n(y + 26.0),
                esc(&meta)
            ));
        }
        for (j, line) in summ.iter().enumerate() {
            parts.push(format!(
                r#"<text class="band-sum" x="{}" y="{}">{}</text>"#,
                n(x0 + 18.0),
                n(y + 46.0 + 15.0 * j as f64),
                esc(line)
            ));
        }
        for (i, d) in owns.iter().enumerate() {
            let dom = m.domains.get(d).cloned().unwrap_or(Value::Null);
            let ents = strs(&dom, "entities");
            let cx = x0 + 20.0 + i as f64 * (cw + 16.0);
            let cy = y + top_h + 6.0;
            let disp = display(&dom);
            parts.push(format!(
                r#"<g class="domain" style="{}" {}><rect x="{cx:.1}" y="{cy:.1}" width="{cw:.1}" height="{}" rx="10"/><rect class="dstripe" x="{cx:.1}" y="{:.1}" width="4" height="{}" rx="2"/><text class="nm" x="{:.1}" y="{:.1}">{}</text>"#,
                dom_style(m, d), tag(m, "domain", d, true, &[]), n(chip_h), cy + 10.0, n(chip_h - 20.0), cx + 14.0, cy + 20.0, esc(d)
            ));
            let cnt = format!(
                "{} cmd · {} evt · {} view · {} actor",
                arr(&dom, "commands").len(),
                arr(&dom, "events").len(),
                arr(&dom, "views").len(),
                arr(&dom, "actors").len()
            );
            parts.push(format!(
                r#"<text class="sub" x="{:.1}" y="{:.1}">{}{}</text>"#,
                cx + 14.0,
                cy + 36.0,
                disp.map(|d| esc(&format!("{d} · "))).unwrap_or_default(),
                esc(&cnt)
            ));
            for (k, e) in ents.iter().enumerate() {
                let ey = cy + 58.0 + 17.0 * k as f64;
                parts.push(format!(r#"<g class="dent" {}><circle cx="{:.1}" cy="{:.1}" r="3"/><text x="{:.1}" y="{ey:.1}">{}</text></g>"#, tag(m, "entity", e, false, &[]), cx + 20.0, ey - 4.0, cx + 30.0, esc(short(e))));
            }
            parts.push("</g>".into());
        }
        parts.push("</g>".into());
        if let Some(c) = cname {
            cpos.insert(c.clone(), (y, bh));
        }
        bands.push((y, bh));
        y += bh;
    }
    let h = y + 30.0;
    let mut flows = Vec::new();
    let mut prev_bottom = 20.0 + band_h;
    for (by, bh) in &bands {
        flows.push(format!(
            r#"<path class="flow" d="M{:.1},{:.1} L{:.1},{:.1}" marker-end="url(#arrow-flow)"/>"#,
            (x0 + x1) / 2.0,
            prev_bottom + 2.0,
            (x0 + x1) / 2.0,
            by - 4.0
        ));
        prev_bottom = by + bh;
    }
    let mut links = Vec::new();
    for (i, a) in actors.iter().enumerate() {
        let may = strs(&m.actors[*a], "may");
        let mut targets: Vec<String> = Vec::new();
        for cmd in &may {
            for c in m.acceptors.get(cmd).into_iter().flatten() {
                if !targets.contains(c) {
                    targets.push(c.clone());
                }
            }
        }
        let (ax, ay) = apos[a];
        let gx = x1 + 26.0 + i as f64 * 22.0;
        for c in &targets {
            let Some((cy, _)) = cpos.get(c) else { continue };
            let ty = cy + 20.0 + 10.0 * i as f64;
            let cnt = may
                .iter()
                .filter(|cmd| m.acceptors.get(*cmd).is_some_and(|v| v.contains(c)))
                .count();
            let rel = [m.a("actor", a), m.a("component", c)];
            links.push(format!(
                r#"<path class="alink" style="--c:{}" d="M{ax:.1},{ay:.1} C{ax:.1},{:.1} {gx:.1},{ay:.1} {gx:.1},{:.1} L{gx:.1},{ty:.1} L{:.1},{ty:.1}" marker-end="url(#arrow-link)" data-ess-rel="{}"><title>{} may send {cnt} command(s) accepted by {}</title></path>"#,
                colour[*a], ay + 30.0, ay + 40.0, x1 + 4.0, esc(&rel.join(" ")), esc(a), esc(c)
            ));
            links.push(format!(
                r#"<text class="alabel" style="fill:{}" x="{:.1}" y="{:.1}">{cnt}</text>"#,
                colour[*a],
                gx + 6.0,
                ty - 5.0
            ));
        }
        let unaccepted = may.iter().filter(|c| !m.acceptors.contains_key(*c)).count();
        if unaccepted > 0 {
            links.push(format!(r#"<text class="alabel warn" x="{ax:.1}" y="{:.1}" text-anchor="middle">{unaccepted} of its commands accepted by no component</text>"#, ay + 16.0));
        }
    }
    let svg = format!(
        r#"<svg class="arch" viewBox="0 0 {} {}" style="max-width:{}px" role="img" aria-label="Components and domains">{}{}{}</svg>"#,
        n(w),
        n(h),
        n(w),
        flows.concat(),
        parts.concat(),
        links.concat()
    );
    (svg, colour)
}

// ---- entities and relations -----------------------------------------------------------------------

enum RowRef {
    None,
    Rel(Value),
    Upd(Box<Cause>),
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
/// The entity-relation diagram.
pub fn render_er(m: &Model<'_>) -> String {
    let mut rows_by_comp: Vec<(Option<String>, Vec<String>)> = Vec::new();
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
        rows_by_comp.push((Some(c.clone()), ents));
    }
    let placed: BTreeSet<String> = rows_by_comp.iter().flat_map(|(_, es)| es.clone()).collect();
    let rest: Vec<String> = m
        .entities
        .keys()
        .filter(|e| !placed.contains(*e))
        .cloned()
        .collect();
    if !rest.is_empty() {
        rows_by_comp.push((None, rest));
    }
    let id_types: BTreeMap<String, String> = m
        .entities
        .iter()
        .map(|(n, e)| {
            (
                st(g(g(e, "identity"), "type_ref"), "name").to_owned(),
                n.clone(),
            )
        })
        .collect();
    let ent_rows = |en: &str, eo: &Value| -> Vec<(String, String, RowRef)> {
        let ident = g(eo, "identity");
        let mut rows = vec![(
            "id".to_owned(),
            format!(
                "{}: {}",
                st(ident, "name"),
                type_label(g(ident, "type_ref"))
            ),
            RowRef::None,
        )];
        for f in arr(eo, "fields") {
            let fk = id_types.contains_key(st(g(f, "type_ref"), "name"))
                && !st(g(f, "type_ref"), "name").is_empty();
            rows.push((
                if fk { "fk" } else { "f" }.to_owned(),
                format!("{}: {}", st(f, "name"), type_label(g(f, "type_ref"))),
                RowRef::None,
            ));
        }
        for inv in arr(eo, "invariants") {
            let t = inv
                .as_str()
                .map_or_else(|| st(inv, "statement").to_owned(), ToOwned::to_owned);
            let t = if t.is_empty() { inv.to_string() } else { t };
            rows.push(("inv".into(), format!("invariant {t}"), RowRef::None));
        }
        for r in arr(eo, "relations") {
            let via = r
                .get("via")
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
                .map(|v| format!(" · via {v}"))
                .unwrap_or_default();
            rows.push((
                format!(
                    "rel-{}",
                    if st(r, "kind") == "owns" {
                        "own"
                    } else {
                        "ref"
                    }
                ),
                format!(
                    "{} {} → {} · {}{via}",
                    st(r, "kind"),
                    st(r, "name"),
                    short(st(r, "target")),
                    text(g(r, "cardinality"))
                ),
                RowRef::Rel(r.clone()),
            ));
        }
        for c in m.updaters.get(en).into_iter().flatten() {
            let sets = if c.sets.is_empty() {
                String::new()
            } else {
                format!(" sets {}", c.sets.join(", "))
            };
            rows.push((
                "upd".into(),
                format!("Δ {} › {}{sets}", short(&c.command), c.outcome),
                RowRef::Upd(Box::new(c.clone())),
            ));
        }
        for c in m.deleters.get(en).into_iter().flatten() {
            rows.push((
                "upd".into(),
                format!("✕ {} › {} removes it", short(&c.command), c.outcome),
                RowRef::Upd(Box::new(c.clone())),
            ));
        }
        rows
    };
    let longest = m
        .entities
        .iter()
        .flat_map(|(en, eo)| {
            ent_rows(en, eo)
                .into_iter()
                .map(|(_, t, _)| t.chars().count())
        })
        .max()
        .unwrap_or(30);
    let bw = (longest as f64 * 6.7 + 30.0).max(260.0);
    let (gx, gy, row) = (120.0, 96.0, 16.0);
    let x0 = rows_by_comp
        .iter()
        .map(|(c, _)| {
            c.as_deref()
                .unwrap_or("no owning component")
                .chars()
                .count()
        })
        .max()
        .unwrap_or(10) as f64
        * 9.0
        + 36.0;
    let mut boxes: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
    let mut relrow: BTreeMap<(String, String), f64> = BTreeMap::new();
    let mut parts = Vec::new();
    let mut y = 56.0;
    for (c, ents) in &rows_by_comp {
        parts.push(format!(
            r#"<text class="er-col" x="14" y="{}">{}</text>"#,
            n(y + 20.0),
            c.as_deref()
                .map_or_else(|| "no owning component".to_owned(), esc)
        ));
        let mut row_h: f64 = 0.0;
        for (i, e) in ents.iter().enumerate() {
            let x = x0 + i as f64 * (bw + gx);
            let eo = &m.entities[e];
            let rows = ent_rows(e, eo);
            let lc = g(eo, "lifecycle");
            let states = strs(lc, "states");
            let mut badge_rows: Vec<Vec<(String, f64)>> = Vec::new();
            let mut cur: Vec<(String, f64)> = Vec::new();
            let mut cw = 0.0;
            for s in &states {
                let w = text_w(s, 6.4) + 14.0;
                if !cur.is_empty() && cw + w > bw - 24.0 {
                    badge_rows.push(std::mem::take(&mut cur));
                    cw = 0.0;
                }
                cur.push((s.clone(), w));
                cw += w + 5.0;
            }
            if !cur.is_empty() {
                badge_rows.push(cur);
            }
            let h = 44.0 + row * rows.len() as f64 + 8.0 + 22.0 * badge_rows.len() as f64 + 6.0;
            boxes.insert(e.clone(), (x, y, bw, h));
            row_h = row_h.max(h);
            let d = st(eo, "domain");
            parts.push(format!(
                r#"<g class="ent" style="{}" {}><rect x="{}" y="{}" width="{}" height="{}" rx="10"/><rect class="dstripe" x="{}" y="{}" width="4" height="{}" rx="2"/><text class="nm" x="{}" y="{}">{}</text><text class="sub" x="{}" y="{}">{}</text>"#,
                dom_style(m, d), tag(m, "entity", e, true, &[]), n(x), n(y), n(bw), n(h), n(x), n(y + 10.0), n(h - 20.0), n(x + 12.0), n(y + 20.0), esc(short(e)), n(x + 12.0), n(y + 34.0), esc(d)
            ));
            for (k, (cls, t, r)) in rows.iter().enumerate() {
                let ry = y + 54.0 + row * k as f64;
                match r {
                    RowRef::Upd(c2) => parts.push(format!(
                        r#"<text class="row r-{cls}" x="{}" y="{}" {}>{}</text>"#,
                        n(x + 12.0),
                        n(ry),
                        tag(
                            m,
                            "outcome",
                            &format!("{}/{}", c2.command, c2.outcome),
                            false,
                            &cause_anchors(m, c2)
                        ),
                        esc(t)
                    )),
                    RowRef::Rel(r) => {
                        let rid = format!("{e}.{}->{}", st(r, "name"), st(r, "target"));
                        parts.push(format!(
                            r#"<text class="row r-{cls}" x="{}" y="{}" {}>{}</text>"#,
                            n(x + 12.0),
                            n(ry),
                            tag(
                                m,
                                "relation",
                                &rid,
                                false,
                                &[m.a("entity", st(r, "target"))]
                            ),
                            esc(t)
                        ));
                        relrow.insert((e.clone(), st(r, "name").to_owned()), ry - 4.0);
                    }
                    RowRef::None => parts.push(format!(
                        r#"<text class="row r-{cls}" x="{}" y="{}">{}</text>"#,
                        n(x + 12.0),
                        n(ry),
                        esc(t)
                    )),
                }
            }
            let mut sy = y + 44.0 + row * rows.len() as f64 + 20.0;
            let single = states.len() <= 1;
            let terms = strs(lc, "terminal");
            for br in &badge_rows {
                let mut sx = x + 12.0;
                for (s, w) in br {
                    let mut cls = "sbadge".to_owned();
                    if s == st(lc, "initial") {
                        cls.push_str(" initial");
                    }
                    if terms.contains(s) {
                        cls.push_str(" terminal");
                    }
                    parts.push(format!(
                        r#"<g class="{cls}" {}><rect x="{sx:.1}" y="{}" width="{w:.1}" height="17" rx="8"/><text x="{:.1}" y="{}">{}</text></g>"#,
                        tag(m, "state", &format!("{e}#{s}"), single, &[]), n(sy - 12.0), sx + 7.0, n(sy + 0.5), esc(s)
                    ));
                    sx += w + 5.0;
                }
                sy += 22.0;
            }
            parts.push("</g>".into());
        }
        y += row_h + gy;
    }
    let mut rels = Vec::new();
    let mut incoming: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (e, eo) in m.entities {
        for r in arr(eo, "relations") {
            incoming
                .entry(st(r, "target").to_owned())
                .or_default()
                .push((e.clone(), st(r, "name").to_owned()));
        }
    }
    let mut attach: BTreeMap<(String, String), (usize, usize)> = BTreeMap::new();
    for lst in incoming.values() {
        for (i, (e, rn)) in lst.iter().enumerate() {
            attach.insert((e.clone(), rn.clone()), (i, lst.len()));
        }
    }
    let mut y_end = y;
    let mut gap_lanes: BTreeMap<i64, i64> = BTreeMap::new();
    let mut row_lanes: BTreeMap<i64, i64> = BTreeMap::new();
    for (e, eo) in m.entities {
        for r in arr(eo, "relations") {
            let t = st(r, "target");
            let rn = st(r, "name");
            let rid = format!("{e}.{rn}->{t}");
            let via = r
                .get("via")
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty());
            let title = format!(
                "{e} {} {rn} → {t} ({}{})",
                st(r, "kind"),
                text(g(r, "cardinality")),
                via.map(|v| format!(", via {v}")).unwrap_or_default()
            );
            let k = if st(r, "kind") == "owns" {
                "own"
            } else {
                "ref"
            };
            let rel = [m.a("entity", e), m.a("entity", t)];
            let (Some(sb), Some(tb)) = (boxes.get(e), boxes.get(t)) else {
                rels.push(format!(r#"<g {}><text class="rlabel warn" x="14" y="{}">{}: target not drawn</text></g>"#, tag(m, "relation", &rid, true, &[]), n(y_end), esc(&rid)));
                y_end += 18.0;
                continue;
            };
            let ((sx, sy, sw, _), (tx, ty, tw, _)) = (*sb, *tb);
            let y1 = relrow
                .get(&(e.clone(), rn.to_owned()))
                .copied()
                .unwrap_or(sy + 14.0);
            let (i, cnt) = attach[&(e.clone(), rn.to_owned())];
            let d = if e == t {
                format!(
                    "M{},{} C{},{} {},{} {},{}",
                    n(sx + sw),
                    n(y1),
                    n(sx + sw + 50.0),
                    n(y1),
                    n(sx + sw + 50.0),
                    n(sy + 14.0),
                    n(sx + sw),
                    n(sy + 14.0)
                )
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let kl = *gap_lanes.get(&(sx as i64)).unwrap_or(&0);
                #[allow(clippy::cast_possible_truncation)]
                gap_lanes.insert(sx as i64, kl + 1);
                let xx = sx + sw + 18.0 + 12.0 * kl as f64;
                #[allow(clippy::cast_possible_truncation)]
                let kc = *row_lanes.get(&(ty as i64)).unwrap_or(&0);
                #[allow(clippy::cast_possible_truncation)]
                row_lanes.insert(ty as i64, kc + 1);
                let yc = ty - 20.0 - 9.0 * kc as f64;
                let ax = tx + tw * (i + 1) as f64 / (cnt + 1) as f64;
                format!(
                    "M{},{} L{xx:.1},{} L{xx:.1},{yc:.1} L{ax:.1},{yc:.1} L{ax:.1},{}",
                    n(sx + sw),
                    n(y1),
                    n(y1),
                    n(ty)
                )
            };
            rels.push(format!(
                r#"<g class="rel k-{k}" {}><path d="{d}" marker-start="url(#m-{k})" marker-end="url(#arrow-rel)"><title>{}</title></path></g>"#,
                tag(m, "relation", &rid, true, &rel), esc(&title)
            ));
        }
    }
    let maxw = boxes.values().map(|b| b.0 + b.2).fold(600.0_f64, f64::max) + 40.0;
    format!(
        r#"<svg class="er" viewBox="0 0 {} {}" style="max-width:{}px" role="img" aria-label="Entities and relations">{}{}</svg>"#,
        n(maxw),
        n(y_end),
        n(maxw),
        parts.concat(),
        rels.concat()
    )
}

// ---- cards --------------------------------------------------------------------------------------------

fn chip(t: &str, cls: &str) -> String {
    format!(r#"<span class="{cls}" >{}</span>"#, esc(t))
}

/// A chip linking to a declaration.
pub fn ref_chip(m: &Model<'_>, kind: &str, ident: &str, label: Option<&str>, cls: &str) -> String {
    let a = m.a(kind, ident);
    format!(
        r##"<a class="{cls} ref" href="#{}" data-ref="{}" title="{}">{}</a>"##,
        esc(&a),
        esc(&a),
        esc(ident),
        esc(label.unwrap_or_else(|| short(ident)))
    )
}

fn creates_into(m: &Model<'_>, subj: &Value) -> String {
    subj.get("into")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            m.entities
                .get(st(subj, "entity"))
                .map(|e| st(g(e, "lifecycle"), "initial").to_owned())
                .unwrap_or_default()
        })
}

/// Commands whose outcomes branch on a guard, and how many.
pub fn render_branches(m: &Model<'_>) -> (String, usize) {
    let mut out = Vec::new();
    for (cn, c) in m.commands {
        let ocs = arr(c, "outcomes");
        if !ocs
            .iter()
            .any(|o| GUARD_KINDS.contains(&st(g(o, "condition"), "kind")))
        {
            continue;
        }
        let inputs: BTreeMap<&str, &Value> =
            arr(c, "input").iter().map(|f| (st(f, "name"), f)).collect();
        let mut claimed: Vec<String> = Vec::new();
        let mut rows = Vec::new();
        let mut field: Option<String> = None;
        for o in ocs {
            let cond = g(o, "condition");
            let subj = g(o, "subject");
            let tr = g(subj, "transition");
            let eff = st(subj, "effect");
            let target = if tr.is_object() {
                format!(
                    "{}: {} → {}",
                    short(st(subj, "entity")),
                    strs(tr, "from").join(", "),
                    st(tr, "to")
                )
            } else if eff == "creates" {
                format!(
                    "creates {} into {}",
                    short(st(subj, "entity")),
                    creates_into(m, subj)
                )
            } else if eff == "updates" {
                format!("updates {} (no state change)", short(st(subj, "entity")))
            } else if eff == "deletes" {
                format!(
                    "deletes {} (removes the instance)",
                    short(st(subj, "entity"))
                )
            } else if let Some(e) = o.get("error").and_then(Value::as_str) {
                format!("refuses: error {}", short(e))
            } else {
                eff.to_owned()
            };
            let mut sel = Vec::new();
            let mut split = None;
            if GUARD_KINDS.contains(&st(cond, "kind")) {
                match predicate_variants(g(cond, "predicate")) {
                    Some((f, vs)) if st(cond, "kind") == "when" => {
                        if field.is_none() {
                            field = Some(f);
                        }
                        claimed.extend(vs.clone());
                        sel = vs;
                        split = Some(true);
                    }
                    _ => split = Some(false),
                }
            }
            rows.push((o, cond, sel, target, split));
        }
        let mut variants: Vec<String> = Vec::new();
        if let Some(f) = field.as_deref().and_then(|f| inputs.get(f)) {
            let body = m
                .types
                .get(st(g(f, "type_ref"), "name"))
                .map(|t| g(t, "body").clone())
                .unwrap_or(Value::Null);
            if st(&body, "kind") == "enum" {
                variants = arr(&body, "variants")
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map_or_else(|| st(v, "name").to_owned(), ToOwned::to_owned)
                    })
                    .collect();
            }
        }
        let mut h = vec![format!(
            r#"<div class="card branch" style="{}">{}"#,
            dom_style(m, st(c, "domain")),
            card_head(m, "command", cn, None)
        )];
        if !variants.is_empty() {
            let f = field.clone().unwrap_or_default();
            h.push(format!(
                r#"<div class="enum">input <b>{}</b>: {} = {}</div>"#,
                esc(&f),
                esc(&type_label(g(inputs[f.as_str()], "type_ref"))),
                variants
                    .iter()
                    .map(|v| chip(v, "chip v"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }
        h.push("<table class=\"btable\"><tr><th>outcome</th><th>condition</th><th>variants</th><th>effect</th><th>emits</th></tr>".into());
        for (o, cond, sel, target, split) in rows {
            let kind = st(cond, "kind");
            let sel = if kind == "otherwise" && !variants.is_empty() {
                variants
                    .iter()
                    .filter(|v| !claimed.contains(v))
                    .cloned()
                    .collect()
            } else {
                sel
            };
            let vs = if !sel.is_empty() {
                sel.iter()
                    .map(|v| chip(v, "chip v"))
                    .collect::<Vec<_>>()
                    .join(" ")
            } else if split == Some(false) {
                r#"<span class="guardtxt">guard, not an enum split</span>"#.into()
            } else {
                r#"<span class="dim">—</span>"#.into()
            };
            let emits: Vec<String> = strs(o, "emits")
                .iter()
                .map(|e| ref_chip(m, "event", e, None, "chip ev"))
                .collect();
            h.push(format!(
                r#"<tr class="c-{}"><td class="mono">{}</td><td class="mono">{}</td><td>{vs}</td><td class="mono">{}</td><td>{}</td></tr>"#,
                esc(kind), esc(st(o, "name")), esc(&condition_text(cond)), esc(&target), if emits.is_empty() { "<span class=dim>—</span>".to_owned() } else { emits.join(" ") }
            ));
        }
        h.push("</table></div>".into());
        out.push(h.concat());
    }
    let count = out.len();
    (
        if out.is_empty() {
            r#"<p class="dim">No command in the IR has a guarded outcome.</p>"#.into()
        } else {
            out.concat()
        },
        count,
    )
}

/// One card per command, grouped by domain.
pub fn render_commands(m: &Model<'_>, colour: &BTreeMap<String, String>) -> String {
    let mut by_dom: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    let mut dom_order: Vec<String> = Vec::new();
    for (cn, c) in m.commands {
        let d = st(c, "domain").to_owned();
        if !by_dom.contains_key(&d) {
            dom_order.push(d.clone());
        }
        by_dom.entry(d).or_default().push(cn);
    }
    let mut out = Vec::new();
    for d in &dom_order {
        out.push(format!(
            r#"<h3 class="dom-h" style="{}"><i class="dsw"></i>{}</h3><div class="grid">"#,
            dom_style(m, d),
            esc(d)
        ));
        for cn in &by_dom[d] {
            let c = &m.commands[*cn];
            let nm = g(c, "naming");
            let mut h = vec![format!(
                r#"<div class="card cmd" style="{}" {}>{}"#,
                dom_style(m, d),
                tag(m, "command", cn, true, &[]),
                card_head(m, "command", cn, None)
            )];
            let mut meta = Vec::new();
            if let Some(dsp) = nm.get("display").and_then(Value::as_str) {
                meta.push(format!("display “{}”", esc(dsp)));
            }
            if let Some(w) = nm.get("wire").and_then(Value::as_str) {
                meta.push(format!("wire <code>{}</code>", esc(w)));
            }
            if !meta.is_empty() {
                h.push(format!(r#"<div class="meta">{}</div>"#, meta.join(" · ")));
            }
            let actors: Vec<String> = m.senders.get(*cn).into_iter().flatten().map(|a| {
                let aa = m.a("actor", a);
                format!(r##"<a class="chip actor ref" href="#{}" data-ref="{}" style="--c:{}" title="{}">{}</a>"##, esc(&aa), esc(&aa), colour.get(a).map_or("#888", String::as_str), esc(a), esc(short(a)))
            }).collect();
            h.push(format!(
                r#"<div class="kv"><span>sent by</span>{}</div>"#,
                if actors.is_empty() {
                    r#"<span class="warn">no actor may send it</span>"#.to_owned()
                } else {
                    actors.join(" ")
                }
            ));
            let comps: Vec<String> = m
                .acceptors
                .get(*cn)
                .into_iter()
                .flatten()
                .map(|x| ref_chip(m, "component", x, Some(x), "chip comp"))
                .collect();
            h.push(format!(
                r#"<div class="kv"><span>accepted by</span>{}</div>"#,
                if comps.is_empty() {
                    r#"<span class="warn">no component accepts it</span>"#.to_owned()
                } else {
                    comps.join(" ")
                }
            ));
            let ins: Vec<String> = arr(c, "input")
                .iter()
                .map(|f| format!("{}: {}", st(f, "name"), type_label(g(f, "type_ref"))))
                .collect();
            h.push(format!(
                r#"<div class="kv"><span>input</span><code>{}</code></div>"#,
                if ins.is_empty() {
                    "—".into()
                } else {
                    esc(&ins.join(", "))
                }
            ));
            let exs: Vec<String> = obj(c, "examples")
                .iter()
                .map(|(k, v)| format!("{k} = {v}"))
                .collect();
            if !exs.is_empty() {
                h.push(format!(
                    r#"<div class="kv"><span>examples</span><code>{}</code></div>"#,
                    esc(&exs.join(", "))
                ));
            }
            for bn in m.bind_by_command.get(*cn).into_iter().flatten() {
                h.push(format!(
                    r#"<div class="kv"><span>bound from</span>{}</div>"#,
                    ref_chip(m, "binding", bn, Some(bn), "chip")
                ));
            }
            h.push(r#"<ul class="outcomes">"#.into());
            for o in arr(c, "outcomes") {
                let cond = g(o, "condition");
                let ck = st(cond, "kind");
                let subj = g(o, "subject");
                let tr = g(subj, "transition");
                let eff_kind = st(subj, "effect");
                let eff = if eff_kind == "moves" && tr.is_object() {
                    format!(
                        "moves {} {}: {} → {}",
                        short(st(subj, "entity")),
                        st(tr, "name"),
                        strs(tr, "from").join(", "),
                        st(tr, "to")
                    )
                } else if eff_kind == "updates" {
                    format!(
                        "Δ updates {}; its state does not change",
                        short(st(subj, "entity"))
                    )
                } else if eff_kind == "deletes" {
                    format!(
                        "✕ deletes {}; the instance is removed, from any state",
                        short(st(subj, "entity"))
                    )
                } else if !eff_kind.is_empty() {
                    let mut e = format!("{eff_kind} {}", short(st(subj, "entity")));
                    if eff_kind == "creates" {
                        let _ = write!(e, " into {}", creates_into(m, subj));
                    }
                    let from = st(g(subj, "instance"), "from");
                    if !from.is_empty() {
                        let _ = write!(e, " (instance {from})");
                    }
                    e
                } else {
                    String::new()
                };
                let pay: BTreeMap<&str, Vec<String>> = arr(o, "payload")
                    .iter()
                    .map(|p| {
                        (
                            st(p, "event"),
                            arr(p, "fields")
                                .iter()
                                .map(|f| assign_text(f, false))
                                .collect(),
                        )
                    })
                    .collect();
                let em: Vec<String> = strs(o, "emits")
                    .iter()
                    .map(|e| {
                        ref_chip(
                            m,
                            "event",
                            e,
                            Some(&format!(
                                "{}({})",
                                short(e),
                                pay.get(e.as_str())
                                    .map(|v| v.join(", "))
                                    .unwrap_or_default()
                            )),
                            "chip ev",
                        )
                    })
                    .collect();
                let refusal = if REFUSAL_KINDS.contains(&ck) {
                    let what = match ck {
                        "unknown_instance" => "unknown instance",
                        "existing_instance" => "identity taken",
                        _ => "wrong state",
                    };
                    chip(&format!("refusal: {what}"), "chip refusal")
                } else if ck == "external" {
                    chip("external cause", "chip ext")
                } else if GUARD_KINDS.contains(&ck) && !subj.is_object() {
                    chip("refusal: guard", "chip refusal")
                } else {
                    String::new()
                };
                let epay: Vec<String> = arr(o, "error_payload")
                    .iter()
                    .map(|f| assign_text(f, false))
                    .collect();
                let err = o
                    .get("error")
                    .and_then(Value::as_str)
                    .map(|e| {
                        let label = (!epay.is_empty())
                            .then(|| format!("{}({})", short(e), epay.join(", ")));
                        ref_chip(m, "error", e, label.as_deref(), "chip err")
                    })
                    .unwrap_or_default();
                let sets: Vec<String> = arr(o, "sets")
                    .iter()
                    .map(|s| assign_text(s, true))
                    .collect();
                let mut extra = Vec::new();
                if let Some(ts) = o.get("test_strategy").and_then(Value::as_str) {
                    extra.push(format!("test_strategy {ts}"));
                }
                if o.get("complete_refusal").and_then(Value::as_bool) == Some(true) {
                    extra.push("complete_refusal".into());
                }
                if !sets.is_empty() {
                    extra.push(format!("sets {}", sets.join(", ")));
                }
                let oid = format!("{cn}/{}", st(o, "name"));
                h.push(format!(
                    r#"<li {} class="c-{}"><div><b class="mono">{}</b> <span class="cond mono">{}</span> {}</div>{}<div>{refusal}{}{err}</div>{}{}</li>"#,
                    tag(m, "outcome", &oid, true, &[]), esc(ck), esc(st(o, "name")), esc(&condition_text(cond)), src_link(m, "outcome", &oid),
                    if eff.is_empty() { String::new() } else { format!("<div class=mono>{}</div>", esc(&eff)) },
                    em.join(" "),
                    o.get("summary").and_then(Value::as_str).map(|s| format!("<div class=summ>{}</div>", esc(s.trim()))).unwrap_or_default(),
                    if extra.is_empty() { String::new() } else { format!("<div class=dim>{}</div>", esc(&extra.join(" · "))) }
                ));
            }
            h.push("</ul></div>".into());
            out.push(h.concat());
        }
        out.push("</div>".into());
    }
    if out.is_empty() {
        r#"<p class="dim">No commands in the IR.</p>"#.into()
    } else {
        out.concat()
    }
}

/// One card per event.
pub fn render_events(m: &Model<'_>) -> String {
    if m.events.is_empty() {
        return r#"<p class="dim">No events in the IR.</p>"#.into();
    }
    let mut out = vec![r#"<div class="grid">"#.to_owned()];
    for (en, e) in m.events {
        let fields: Vec<String> = arr(e, "fields")
            .iter()
            .map(|f| format!("{}: {}", st(f, "name"), type_label(g(f, "type_ref"))))
            .collect();
        let by: Vec<String> = m
            .emitted_by
            .get(en)
            .into_iter()
            .flatten()
            .map(|(c, o)| {
                ref_chip(
                    m,
                    "outcome",
                    &format!("{c}/{o}"),
                    Some(&format!("{} › {o}", short(c))),
                    "chip",
                )
            })
            .collect();
        let pubs: Vec<String> = m
            .publishers
            .get(en)
            .into_iter()
            .flatten()
            .map(|p| ref_chip(m, "component", p, Some(p), "chip comp"))
            .collect();
        let binds: Vec<String> = m
            .bind_by_event
            .get(en)
            .into_iter()
            .flatten()
            .map(|b| ref_chip(m, "binding", b, Some(b), "chip"))
            .collect();
        out.push(format!(
            r#"<div class="card ev" style="{}" {}>{}<div class="kv"><span>fields</span><code>{}</code></div><div class="kv"><span>emitted by</span>{}</div><div class="kv"><span>published by</span>{}</div>{}</div>"#,
            dom_style(m, st(e, "domain")), tag(m, "event", en, true, &[]), card_head(m, "event", en, None),
            if fields.is_empty() { "—".into() } else { esc(&fields.join(", ")) },
            if by.is_empty() { "<span class=warn>no outcome emits it</span>".into() } else { by.join(" ") },
            if pubs.is_empty() { "<span class=warn>no component publishes it</span>".into() } else { pubs.join(" ") },
            if binds.is_empty() { String::new() } else { format!(r#"<div class="kv"><span>binds to</span>{}</div>"#, binds.join(" ")) }
        ));
    }
    out.push("</div>".into());
    out.concat()
}

/// One card per view.
pub fn render_views(m: &Model<'_>) -> String {
    if m.views.is_empty() {
        return r#"<p class="dim">No views in the IR.</p>"#.into();
    }
    let mut out = vec![r#"<div class="grid">"#.to_owned()];
    for (vn, v) in m.views {
        let nm = g(v, "naming");
        let agg = g(v, "aggregation");
        let fns = obj(agg, "functions");
        let ftext = |f: &Value| {
            let computed = fns
                .get(st(f, "name"))
                .map(|fnv| {
                    format!(
                        " = {}({})",
                        st(fnv, "function"),
                        st(g(fnv, "input"), "name")
                    )
                })
                .unwrap_or_default();
            format!(
                "{}: {}{computed}",
                st(f, "name"),
                type_label(g(f, "type_ref"))
            )
        };
        let fields: Vec<String> = arr(v, "fields").iter().map(ftext).collect();
        let group_html = if agg.is_object() {
            let gb = strs(agg, "group_by");
            format!(
                r#"<div class="kv"><span>group_by</span><code>{}</code></div>"#,
                esc(&if gb.is_empty() {
                    "none (one row)".to_owned()
                } else {
                    gb.join(", ")
                })
            )
        } else {
            String::new()
        };
        let filt = g(v, "filter");
        let ftxt = (!filt.is_null()).then(|| predicate_text(filt));
        let params: Vec<String> = arr(v, "params")
            .iter()
            .map(|p| format!("{}: {}", st(p, "name"), type_label(g(p, "type_ref"))))
            .collect();
        let params_html = if params.is_empty() {
            String::new()
        } else {
            format!(
                r#"<div class="kv"><span>params</span><code>{}</code></div>"#,
                esc(&params.join(", "))
            )
        };
        let order: Vec<String> = arr(v, "order_by").iter().map(text).collect();
        let meta: Vec<String> = [
            nm.get("display")
                .and_then(Value::as_str)
                .map(|d| format!("display “{d}”")),
            nm.get("wire")
                .and_then(Value::as_str)
                .map(|w| format!("wire {w}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        let order_html = if order.is_empty() {
            "<span class=dim>none (no declared order)</span>".to_owned()
        } else {
            order
                .iter()
                .map(|o| {
                    chip(
                        &format!("{}{o}", if o.ends_with(" desc") { "↓ " } else { "↑ " }),
                        "chip ord",
                    )
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        out.push(format!(
            r#"<div class="card view" style="{}" {}>{}{}<div class="kv"><span>source</span>{}</div>{params_html}<div class="kv"><span>filter</span>{}</div><div class="kv"><span>order_by</span>{order_html}</div>{group_html}<div class="kv"><span>fields</span><code>{}</code></div><div class="kv"><span>consistency</span><code>{}</code><span>assertion_style</span><code>{}</code></div></div>"#,
            dom_style(m, st(v, "domain")), tag(m, "view", vn, true, &[m.a("entity", st(v, "source"))]), card_head(m, "view", vn, None),
            if meta.is_empty() { String::new() } else { format!("<div class=meta>{}</div>", esc(&meta.join(" · "))) },
            ref_chip(m, "entity", st(v, "source"), None, "chip"),
            ftxt.map_or_else(|| "<span class=dim>none (every instance)</span>".to_owned(), |t| format!("<code class=filter>{}</code>", esc(&t))),
            if fields.is_empty() { "—".into() } else { esc(&fields.join(", ")) },
            esc(v.get("consistency").and_then(Value::as_str).unwrap_or("—")),
            esc(v.get("assertion_style").and_then(Value::as_str).unwrap_or("—"))
        ));
    }
    out.push("</div>".into());
    out.concat()
}

/// Bindings, conversions and workloads.
pub fn render_interactions(m: &Model<'_>) -> String {
    let mut out = Vec::new();
    if !m.bindings.is_empty() {
        out.push(r#"<h3 class="dom-h">bindings</h3><div class="grid">"#.to_owned());
        for (bn, b) in m.bindings {
            let maps: String = arr(b, "mapping")
                .iter()
                .map(|mp| {
                    format!(
                        r#"<li class="mono">{}{}</li>"#,
                        esc(&assign_text(mp, true)),
                        mp.get("conversion")
                            .and_then(Value::as_str)
                            .map(|c| format!(r#"<div class="summ">conversion: {}</div>"#, esc(c)))
                            .unwrap_or_default()
                    )
                })
                .collect();
            let rel = [
                m.a("event", st(b, "event")),
                m.a("command", st(b, "command")),
            ];
            out.push(format!(
                r#"<div class="card bind" {}>{}<div class="summ">{}</div><div class="kv"><span>on event</span>{}</div><div class="kv"><span>sends</span>{}</div><div class="kv"><span>delivery</span><code>{}</code><span>failure</span><code>{}</code></div>{}<ul class="outcomes">{maps}</ul></div>"#,
                tag(m, "binding", bn, true, &rel), card_head(m, "binding", bn, Some(bn)), esc(&summary(b)),
                ref_chip(m, "event", st(b, "event"), None, "chip ev"), ref_chip(m, "command", st(b, "command"), None, "chip"),
                esc(b.get("delivery").and_then(Value::as_str).unwrap_or("—")), esc(b.get("failure").and_then(Value::as_str).unwrap_or("—")),
                b.get("escalation").and_then(Value::as_str).map(|e| format!(r#"<div class="kv"><span>escalation</span>{}</div>"#, ref_chip(m, "event", e, None, "chip ev"))).unwrap_or_default()
            ));
        }
        out.push("</div>".into());
    }
    if !m.conversions.is_empty() {
        out.push(r#"<h3 class="dom-h">conversions</h3><div class="grid">"#.to_owned());
        for c in m.conversions {
            out.push(format!(
                r#"<div class="card conv"><div class="card-h"><span class="nm">{} → {}</span><span class="sub">{} → {}</span></div><div class="summ">{}</div></div>"#,
                esc(&type_label(g(c, "from"))), esc(&type_label(g(c, "to"))), esc(st(g(c, "from"), "name")), esc(st(g(c, "to"), "name")), esc(st(c, "because"))
            ));
        }
        out.push("</div>".into());
    }
    if !m.workloads.is_empty() {
        out.push(r#"<h3 class="dom-h">workloads</h3><div class="grid">"#.to_owned());
        for (wn, w) in m.workloads {
            let rep = g(w, "replicas");
            let req: Vec<String> = arr(w, "requires")
                .iter()
                .map(|r| chip(&format!("{}: {}", st(r, "kind"), st(r, "name")), "chip"))
                .collect();
            out.push(format!(
                r#"<div class="card work" {}>{}<div class="kv"><span>component</span>{}</div><div class="kv"><span>replicas</span><code>min {} · max {}</code><span>stateless</span><code>{}</code></div><div class="kv"><span>requires</span>{}</div></div>"#,
                tag(m, "workload", wn, true, &[m.a("component", st(w, "component"))]), card_head(m, "workload", wn, Some(wn)),
                ref_chip(m, "component", st(w, "component"), Some(st(w, "component")), "chip comp"),
                esc(&text(g(rep, "min"))), esc(&text(g(rep, "max"))), esc(&text(g(w, "stateless"))),
                if req.is_empty() { "—".into() } else { req.join(" ") }
            ));
        }
        out.push("</div>".into());
    }
    if out.is_empty() {
        r#"<p class="dim">The IR declares no bindings, conversions or workloads.</p>"#.into()
    } else {
        out.concat()
    }
}

/// Error cards and the types table.
pub fn render_errors_types(m: &Model<'_>) -> String {
    let mut out = vec![r#"<div class="grid">"#.to_owned()];
    for (en, e) in m.errors {
        let fields: Vec<String> = arr(e, "fields")
            .iter()
            .map(|f| format!("{}: {}", st(f, "name"), type_label(g(f, "type_ref"))))
            .collect();
        out.push(format!(
            r#"<div class="card err" style="{}" {}>{}<div class="summ">{}</div><div class="kv"><span>fields</span><code>{}</code></div></div>"#,
            dom_style(m, st(e, "domain")), tag(m, "error", en, true, &[]), card_head(m, "error", en, None), esc(st(e, "summary").trim()),
            if fields.is_empty() { "—".into() } else { esc(&fields.join(", ")) }
        ));
    }
    out.push(r#"</div><h3 class="dom-h">types</h3><table class="types"><tr><th>type</th><th>kind</th><th>body</th><th>source</th></tr>"#.into());
    for (tn, t) in m.types {
        let b = g(t, "body");
        let body = match st(b, "kind") {
            "enum" => arr(b, "variants")
                .iter()
                .map(|v| {
                    chip(
                        &v.as_str()
                            .map_or_else(|| st(v, "name").to_owned(), ToOwned::to_owned),
                        "chip v",
                    )
                })
                .collect::<Vec<_>>()
                .join(" "),
            "newtype" => {
                let inv = arr(b, "invariants");
                format!(
                    "<code>of {}</code>{}",
                    esc(&type_label(g(b, "of"))),
                    if inv.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " <code>invariants {}</code>",
                            esc(&Value::Array(inv.to_vec()).to_string())
                        )
                    }
                )
            }
            _ => format!("<code>{}</code>", esc(&b.to_string())),
        };
        let link = src_link(m, "type", tn);
        let link = if link.is_empty() {
            r#"<span class="dim">implicit (no authored line)</span>"#.to_owned()
        } else {
            link
        };
        out.push(format!(r#"<tr {}><td class="mono">{}</td><td class="mono">{}</td><td>{body}</td><td>{link}</td></tr>"#, tag(m, "type", tn, true, &[]), esc(tn), esc(st(b, "kind"))));
    }
    out.push("</table>".into());
    out.concat()
}

const KIND_ORDER: [&str; 16] = [
    "system",
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
    "binding",
    "workload",
];

/// The source index: every located declaration and its line.
pub fn render_source_index(m: &Model<'_>, missing: usize) -> String {
    let mut rows: Vec<&(String, String)> = m.src.keys().collect();
    rows.sort_by_key(|(k, i)| {
        (
            KIND_ORDER.iter().position(|x| x == k).unwrap_or(99),
            i.clone(),
        )
    });
    let mut out = vec![format!(
        r#"<p class="lede">{} declarations located, {missing} not located. Each link opens the authored line at the rendered ref.</p><table class="srcidx"><tr><th>kind</th><th>declaration</th><th>source</th></tr>"#,
        rows.len()
    )];
    for (kind, ident) in rows {
        let a = m.a(kind, ident);
        out.push(format!(r##"<tr><td class="mono">{}</td><td><a class="mono ref" href="#{}" data-ref="{}">{}</a></td><td>{}</td></tr>"##, esc(kind), esc(&a), esc(&a), esc(ident), src_link(m, kind, ident)));
    }
    out.push("</table>".into());
    out.concat()
}

/// The domain chips with their colours.
pub fn domain_legend(m: &Model<'_>) -> String {
    let items: String = m
        .domains
        .keys()
        .map(|d| {
            let a = m.a("domain", d);
            format!(r##"<a class="dl ref" href="#{}" data-ref="{}" style="{}"><i class="dsw"></i>{}</a>"##, esc(&a), esc(&a), dom_style(m, d), esc(d))
        })
        .collect();
    format!(r#"<div class="dlegend">{items}</div>"#)
}

/// The legend section.
pub fn legend_html(m: &Model<'_>) -> String {
    let ln = |cls: &str, extra: &str| {
        format!(
            r#"<svg viewBox="0 0 64 22" class="lifecycle"><path class="edge {cls}" d="M4,11 L56,11" {extra}/></svg>"#
        )
    };
    let items: Vec<(String, &str)> = vec![
        (r#"<svg viewBox="0 0 64 22" class="lifecycle"><g class="state initial"><rect class="body" x="4" y="3" width="56" height="16" rx="8"/></g></svg>"#.into(), "default initial state (IR <code>lifecycle.initial</code>); a <code>creates</code> outcome can explicitly name another state with <code>into</code>"),
        (r#"<svg viewBox="0 0 64 22" class="lifecycle"><g class="state terminal"><rect class="ring" x="2" y="1" width="60" height="20" rx="10"/><rect class="body" x="6" y="4" width="52" height="14" rx="7"/></g></svg>"#.into(), "terminal state (IR <code>lifecycle.terminal</code>)"),
        (ln("k-unconditional", r#"marker-end="url(#arrow-unconditional)""#), "transition moved by an <code>otherwise</code> outcome of a command with no guard"),
        (ln("k-guarded", r#"marker-end="url(#arrow-guarded)""#), "transition moved by a guarded outcome (<code>when</code> or <code>subject_predicate</code>)"),
        (ln("k-default", r#"marker-end="url(#arrow-default)""#), "transition moved by the <code>otherwise</code> outcome of a branching command"),
        (ln("k-none", r#"marker-end="url(#arrow-none)""#), "transition no command outcome moves"),
        (ln("k-entry", r#"marker-end="url(#arrow-entry)""#), "creation: a <code>creates</code> outcome"),
        (r#"<svg viewBox="0 0 64 22"><text x="4" y="15" font-size="11" class="legend-refuse">refused if</text></svg>"#.into(), "edge line naming a guard outcome of the same command that refuses instead of moving"),
        (r#"<svg viewBox="0 0 64 22" class="lifecycle"><g class="upd"><rect x="4" y="3" width="56" height="16" rx="5"/><text x="10" y="15">Δ</text></g></svg>"#.into(), "<code>updates</code> outcome: fields change, the state does not"),
        (r#"<svg viewBox="0 0 64 22"><circle cx="32" cy="11" r="7" class="legend-token"/></svg>"#.into(), "token: one instance walking the lifecycle (least-walked tour, or a chosen shortest path)"),
        (r#"<svg viewBox="0 0 64 22" class="er"><g class="rel k-own"><path d="M8,11 L56,11" marker-start="url(#m-own)" marker-end="url(#arrow-rel)"/></g></svg>"#.into(), "relation <code>owns</code>"),
        (r#"<svg viewBox="0 0 64 22" class="er"><g class="rel k-ref"><path d="M8,11 L56,11" marker-start="url(#m-ref)" marker-end="url(#arrow-rel)"/></g></svg>"#.into(), "relation <code>references</code>"),
        (r#"<svg viewBox="0 0 64 22"><path class="flow" d="M4,11 L56,11"/></svg>"#.into(), "architecture: band order follows cross-component entity relations"),
        (r#"<span class="chip refusal">refusal</span>"#.into(), "outcome that refuses: <code>wrong_state</code>, <code>unknown_instance</code> or a refusing guard"),
    ];
    let body: String = items
        .iter()
        .map(|(gfx, t)| format!("<div>{gfx}<span>{t}</span></div>"))
        .collect();
    format!(
        r#"<div class="legend">{body}</div><h3 class="dom-h">domains</h3>{}"#,
        domain_legend(m)
    )
}

/// The live panel beside a lifecycle.
pub fn live_panel(svg_id: &str, ename: &str, paths: &[Value]) -> String {
    let rows = [
        ("state", "state"),
        ("transition", "transition"),
        ("command", "command"),
        ("outcome", "outcome"),
        ("condition", "condition"),
        ("actor", "sent by"),
        ("component", "accepted by"),
        ("emits", "emits"),
    ];
    let dl: String = rows
        .iter()
        .map(|(r, l)| format!(r#"<dt>{l}</dt><dd data-r="{r}">—</dd>"#))
        .collect();
    let mut opts =
        r#"<option value="tour">every transition (least-walked tour)</option>"#.to_owned();
    for (i, p) in paths.iter().enumerate() {
        let _ = write!(
            opts,
            r#"<option value="{i}">to {} ({} steps)</option>"#,
            esc(st(p, "to")),
            arr(p, "edges").len()
        );
    }
    format!(
        r#"<div class="panel live" id="{svg_id}-panel"><h4>{} · live</h4><dl>{dl}</dl><div class="covered" data-r="cov"></div><div class="bar"><i></i></div><label class="lbl">path <select data-a="path">{opts}</select></label><div class="lbl">speed {}</div><div class="ctrl"><button data-a="play">Play</button><button data-a="step">Step</button><button data-a="reset">Reset</button></div></div>"#,
        esc(short(ename)),
        speed_control("speed")
    )
}

/// The 0.5×–8× speed control.
pub fn speed_control(attr: &str) -> String {
    let b: String = ["0.5", "1", "2", "4", "8"]
        .iter()
        .map(|v| {
            format!(
                r#"<button type="button" data-speed="{v}"{}>{v}×</button>"#,
                if *v == "1" { " class=on" } else { "" }
            )
        })
        .collect();
    format!(r#"<div class="speedseg" data-a="{attr}" role="group" aria-label="Speed">{b}</div>"#)
}
