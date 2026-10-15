//! SVG import: paths, rectangles, circles and ellipses become Vectro shapes.

use crate::{FillRule, Shape, ShapeKind, SubPath, Vertex};

type P = (f64, f64);

/// A 2D affine transform [a c e; b d f].
#[derive(Clone, Copy, Debug)]
struct M([f64; 6]);

impl M {
    const ID: M = M([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    fn mul(self, o: M) -> M {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = o.0;
        M([a * a2 + c * b2, b * a2 + d * b2, a * c2 + c * d2, b * c2 + d * d2, a * e2 + c * f2 + e, b * e2 + d * f2 + f])
    }
    fn apply(self, (x, y): P) -> P {
        let [a, b, c, d, e, f] = self.0;
        (a * x + c * y + e, b * x + d * y + f)
    }
    /// Rotation-only (plus translation) transforms keep rectangles as rectangles.
    fn rotation(self) -> Option<f64> {
        let [a, b, c, d, ..] = self.0;
        let scale = (a * a + b * b).sqrt();
        ((scale - 1.0).abs() < 1e-6 && (a - d).abs() < 1e-6 && (b + c).abs() < 1e-6).then(|| b.atan2(a).to_degrees())
    }
}

fn numbers(s: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_digit() || c == b'-' || c == b'+' || c == b'.' {
            let start = i;
            i += 1;
            let mut dot = c == b'.';
            while i < b.len() {
                let ch = b[i];
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == b'.' && !dot {
                    dot = true;
                    i += 1;
                } else if (ch == b'e' || ch == b'E') && i + 1 < b.len() {
                    i += 1;
                    if b[i] == b'-' || b[i] == b'+' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(v) = s[start..i].parse() {
                out.push(v);
            }
        } else {
            i += 1;
        }
    }
    out
}

fn parse_transform(s: &str) -> M {
    let mut m = M::ID;
    for part in s.split(')') {
        let Some((name, args)) = part.split_once('(') else { continue };
        let v = numbers(args);
        let t = match (name.trim().trim_start_matches(',').trim(), v.as_slice()) {
            ("translate", [x]) => M([1.0, 0.0, 0.0, 1.0, *x, 0.0]),
            ("translate", [x, y, ..]) => M([1.0, 0.0, 0.0, 1.0, *x, *y]),
            ("scale", [s]) => M([*s, 0.0, 0.0, *s, 0.0, 0.0]),
            ("scale", [x, y, ..]) => M([*x, 0.0, 0.0, *y, 0.0, 0.0]),
            ("rotate", [a, rest @ ..]) => {
                let (s, c) = a.to_radians().sin_cos();
                let r = M([c, s, -s, c, 0.0, 0.0]);
                match rest {
                    [cx, cy, ..] => M([1.0, 0.0, 0.0, 1.0, *cx, *cy]).mul(r).mul(M([1.0, 0.0, 0.0, 1.0, -cx, -cy])),
                    _ => r,
                }
            }
            ("matrix", [a, b, c, d, e, f]) => M([*a, *b, *c, *d, *e, *f]),
            _ => M::ID,
        };
        m = m.mul(t);
    }
    m
}

/// One cubic segment.
type Seg = (P, P, P, P);

/// Parse SVG path data into subpaths of cubic segments, with a closed flag.
fn parse_d(d: &str) -> Vec<(Vec<Seg>, bool)> {
    let mut out: Vec<(Vec<Seg>, bool)> = Vec::new();
    let mut segs: Vec<Seg> = Vec::new();
    let (mut cur, mut start): (P, P) = ((0.0, 0.0), (0.0, 0.0));
    let mut last_c2: Option<P> = None; // for S
    let mut last_q: Option<P> = None; // for T
    let flush = |segs: &mut Vec<Seg>, closed: bool, out: &mut Vec<(Vec<Seg>, bool)>| {
        if !segs.is_empty() {
            out.push((std::mem::take(segs), closed));
        }
    };

    // Split into (command, args) chunks.
    let mut chunks: Vec<(char, Vec<f64>)> = Vec::new();
    let mut cmd_start = None;
    for (i, ch) in d.char_indices() {
        if ch.is_ascii_alphabetic() && ch != 'e' && ch != 'E' {
            if let Some((c, s)) = cmd_start {
                chunks.push((c, numbers(&d[s..i])));
            }
            cmd_start = Some((ch, i + 1));
        }
    }
    if let Some((c, s)) = cmd_start {
        chunks.push((c, numbers(&d[s..])));
    }

    let line = |a: P, b: P| -> Seg { (a, a, b, b) };
    for (cmd, args) in chunks {
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        let off = |p: P, cur: P| if rel { (p.0 + cur.0, p.1 + cur.1) } else { p };
        match up {
            'Z' => {
                if (cur.0 - start.0).abs() > 1e-9 || (cur.1 - start.1).abs() > 1e-9 {
                    segs.push(line(cur, start));
                }
                cur = start;
                flush(&mut segs, true, &mut out);
                last_c2 = None;
                last_q = None;
                continue;
            }
            'M' => {
                for (i, xy) in args.chunks_exact(2).enumerate() {
                    let p = off((xy[0], xy[1]), cur);
                    if i == 0 {
                        flush(&mut segs, false, &mut out);
                        start = p;
                    } else {
                        segs.push(line(cur, p));
                    }
                    cur = p;
                }
            }
            'L' => {
                for xy in args.chunks_exact(2) {
                    let p = off((xy[0], xy[1]), cur);
                    segs.push(line(cur, p));
                    cur = p;
                }
            }
            'H' => {
                for x in args {
                    let p = (if rel { cur.0 + x } else { x }, cur.1);
                    segs.push(line(cur, p));
                    cur = p;
                }
            }
            'V' => {
                for y in args {
                    let p = (cur.0, if rel { cur.1 + y } else { y });
                    segs.push(line(cur, p));
                    cur = p;
                }
            }
            'C' => {
                for v in args.chunks_exact(6) {
                    let (c1, c2, p) = (off((v[0], v[1]), cur), off((v[2], v[3]), cur), off((v[4], v[5]), cur));
                    segs.push((cur, c1, c2, p));
                    last_c2 = Some(c2);
                    cur = p;
                }
                last_q = None;
                continue;
            }
            'S' => {
                for v in args.chunks_exact(4) {
                    let c1 = last_c2.map(|c| (2.0 * cur.0 - c.0, 2.0 * cur.1 - c.1)).unwrap_or(cur);
                    let (c2, p) = (off((v[0], v[1]), cur), off((v[2], v[3]), cur));
                    segs.push((cur, c1, c2, p));
                    last_c2 = Some(c2);
                    cur = p;
                }
                last_q = None;
                continue;
            }
            'Q' | 'T' => {
                let step = if up == 'Q' { 4 } else { 2 };
                for v in args.chunks_exact(step) {
                    let q = if up == 'Q' {
                        off((v[0], v[1]), cur)
                    } else {
                        last_q.map(|c| (2.0 * cur.0 - c.0, 2.0 * cur.1 - c.1)).unwrap_or(cur)
                    };
                    let p = off((v[step - 2], v[step - 1]), cur);
                    let c1 = (cur.0 + 2.0 / 3.0 * (q.0 - cur.0), cur.1 + 2.0 / 3.0 * (q.1 - cur.1));
                    let c2 = (p.0 + 2.0 / 3.0 * (q.0 - p.0), p.1 + 2.0 / 3.0 * (q.1 - p.1));
                    segs.push((cur, c1, c2, p));
                    last_q = Some(q);
                    cur = p;
                }
                last_c2 = None;
                continue;
            }
            'A' => {
                for v in args.chunks_exact(7) {
                    let p = off((v[5], v[6]), cur);
                    segs.extend(arc(cur, v[0], v[1], v[2], v[3] != 0.0, v[4] != 0.0, p));
                    cur = p;
                }
            }
            _ => {}
        }
        last_c2 = None;
        last_q = None;
    }
    flush(&mut segs, false, &mut out);
    out
}

/// SVG elliptical arc to cubic segments (endpoint parameterisation, SVG spec F.6).
fn arc(p0: P, rx: f64, ry: f64, phi_deg: f64, large: bool, sweep: bool, p1: P) -> Vec<Seg> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-9 || ry < 1e-9 || (p0.0 - p1.0).abs() + (p0.1 - p1.1).abs() < 1e-9 {
        return vec![(p0, p0, p1, p1)];
    }
    let (sp, cp) = phi_deg.to_radians().sin_cos();
    let (dx, dy) = ((p0.0 - p1.0) / 2.0, (p0.1 - p1.1) / 2.0);
    let (x1, y1) = (cp * dx + sp * dy, -sp * dx + cp * dy);
    let lam = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lam > 1.0 {
        rx *= lam.sqrt();
        ry *= lam.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut co = (num / den).max(0.0).sqrt();
    if large == sweep {
        co = -co;
    }
    let (cx1, cy1) = (co * rx * y1 / ry, -co * ry * x1 / rx);
    let (cx, cy) = (cp * cx1 - sp * cy1 + (p0.0 + p1.0) / 2.0, sp * cx1 + cp * cy1 + (p0.1 + p1.1) / 2.0);
    let ang = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let a = (ux * vx + uy * vy) / ((ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt());
        let s = if ux * vy - uy * vx < 0.0 { -1.0 } else { 1.0 };
        s * a.clamp(-1.0, 1.0).acos()
    };
    let t1 = ang(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && dt > 0.0 {
        dt -= 2.0 * std::f64::consts::PI;
    } else if sweep && dt < 0.0 {
        dt += 2.0 * std::f64::consts::PI;
    }
    let n = (dt.abs() / (std::f64::consts::PI / 2.0)).ceil().max(1.0) as usize;
    let step = dt / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let pt = |t: f64| {
        let (s, c) = t.sin_cos();
        (cx + rx * c * cp - ry * s * sp, cy + rx * c * sp + ry * s * cp)
    };
    let deriv = |t: f64| {
        let (s, c) = t.sin_cos();
        (-rx * s * cp - ry * c * sp, -rx * s * sp + ry * c * cp)
    };
    (0..n)
        .map(|i| {
            let (a, b) = (t1 + step * i as f64, t1 + step * (i + 1) as f64);
            let (pa, pb, da, db) = (pt(a), pt(b), deriv(a), deriv(b));
            let start = if i == 0 { p0 } else { pa };
            let end = if i + 1 == n { p1 } else { pb };
            (start, (pa.0 + k * da.0, pa.1 + k * da.1), (pb.0 - k * db.0, pb.1 - k * db.1), end)
        })
        .collect()
}

/// Turn cubic segments into vertices with in/out handles.
fn to_subpath(segs: &[Seg], closed: bool, m: M) -> SubPath {
    let mut pts: Vec<(P, P, P)> = Vec::new(); // (in, point, out)
    for (i, &(p0, c1, c2, p1)) in segs.iter().enumerate() {
        if i == 0 {
            pts.push((p0, p0, c1));
        } else {
            pts.last_mut().unwrap().2 = c1;
        }
        pts.push((c2, p1, p1));
    }
    if closed && pts.len() > 1 {
        let (lin, lp, _) = *pts.last().unwrap();
        let first = pts[0].1;
        if (lp.0 - first.0).abs() < 1e-6 && (lp.1 - first.1).abs() < 1e-6 {
            pts.pop();
            pts[0].0 = lin;
        }
    }
    let f = |p: P| {
        let (x, y) = m.apply(p);
        (x as f32, y as f32)
    };
    SubPath {
        closed,
        points: pts
            .into_iter()
            .map(|(i, p, o)| {
                let ((x, y), (in_x, in_y), (out_x, out_y)) = (f(p), f(i), f(o));
                Vertex { x, y, in_x, in_y, out_x, out_y }
            })
            .collect(),
    }
}

fn colour(node: &roxmltree::Node, inherited: Option<u32>) -> Option<u32> {
    let style = node.attribute("style").unwrap_or("");
    let from_style = style.split(';').find_map(|kv| kv.split_once(':').filter(|(k, _)| k.trim() == "fill").map(|(_, v)| v.trim()));
    let raw = from_style.or(node.attribute("fill"));
    let opacity = node.attribute("fill-opacity").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0)
        * node.attribute("opacity").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0);
    let rgb = match raw.map(str::trim) {
        None => return inherited,
        Some("none") | Some("transparent") => return None,
        Some("white") => 0xffffff,
        Some("black") => 0x000000,
        Some(h) if h.starts_with('#') && h.len() == 7 => u32::from_str_radix(&h[1..], 16).ok()?,
        Some(h) if h.starts_with('#') && h.len() == 4 => {
            let v = u32::from_str_radix(&h[1..], 16).ok()?;
            let (r, g, b) = ((v >> 8) & 15, (v >> 4) & 15, v & 15);
            (r * 17) << 16 | (g * 17) << 8 | (b * 17)
        }
        _ => 0x000000,
    };
    Some(((opacity.clamp(0.0, 1.0) * 255.0).round() as u32) << 24 | rgb)
}

pub struct Imported {
    pub width: f32,
    pub height: f32,
    pub shapes: Vec<Shape>,
}

/// Import an SVG document. Shapes come back in painting order (back first), with ids from 1.
pub fn import(svg: &str) -> Result<Imported, String> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| format!("not valid SVG: {e}"))?;
    let root = doc.root_element();
    let vb = root.attribute("viewBox").map(numbers).filter(|v| v.len() == 4);
    let (vx, vy) = vb.as_ref().map(|v| (v[0], v[1])).unwrap_or((0.0, 0.0));
    let width = vb.as_ref().map(|v| v[2]).or_else(|| root.attribute("width").and_then(|w| numbers(w).first().copied())).unwrap_or(500.0);
    let height = vb.as_ref().map(|v| v[3]).or_else(|| root.attribute("height").and_then(|h| numbers(h).first().copied())).unwrap_or(500.0);
    let base = M([1.0, 0.0, 0.0, 1.0, -vx, -vy]);

    let mut shapes = Vec::new();
    walk(root, base, None, &mut shapes);
    for (i, s) in shapes.iter_mut().enumerate() {
        s.id = i as u32 + 1;
    }
    Ok(Imported { width: width as f32, height: height as f32, shapes })
}

fn walk(node: roxmltree::Node, parent: M, fill: Option<u32>, out: &mut Vec<Shape>) {
    for child in node.children().filter(|n| n.is_element()) {
        let m = parent.mul(child.attribute("transform").map(parse_transform).unwrap_or(M::ID));
        let fill = colour(&child, fill.or(Some(0xff000000)));
        let num = |name: &str| child.attribute(name).and_then(|v| numbers(v).first().copied()).unwrap_or(0.0);
        let tag = child.tag_name().name();
        match tag {
            "g" | "svg" => walk(child, m, fill, out),
            "defs" | "clipPath" | "mask" | "style" | "title" => {}
            _ => {
                let Some(fill) = fill else { continue };
                let rule = match child.attribute("fill-rule") {
                    Some("evenodd") => FillRule::EvenOdd,
                    _ => FillRule::NonZero,
                };
                let shape = match tag {
                    "rect" => {
                        let (x, y, w, h) = (num("x"), num("y"), num("width"), num("height"));
                        let rx = child.attribute("rx").or(child.attribute("ry")).and_then(|v| numbers(v).first().copied()).unwrap_or(0.0);
                        match m.rotation() {
                            Some(rot) => {
                                let (cx, cy) = m.apply((x + w / 2.0, y + h / 2.0));
                                Some(basic(ShapeKind::Rectangle, "Rectangle", cx, cy, w, h, rot, rx, fill))
                            }
                            None => Some(path_shape(&parse_d(&format!("M{x} {y}H{}V{}H{x}Z", x + w, y + h)), m, fill, rule)),
                        }
                    }
                    "circle" | "ellipse" => {
                        let (rx, ry) = if tag == "circle" { (num("r"), num("r")) } else { (num("rx"), num("ry")) };
                        let (cx, cy) = m.apply((num("cx"), num("cy")));
                        Some(basic(ShapeKind::Ellipse, "Ellipse", cx, cy, rx * 2.0, ry * 2.0, m.rotation().unwrap_or(0.0), 0.0, fill))
                    }
                    "path" => child.attribute("d").map(|d| path_shape(&parse_d(d), m, fill, rule)),
                    "polygon" | "polyline" => child.attribute("points").map(|p| {
                        let v = numbers(p);
                        let mut d = String::new();
                        for (i, xy) in v.chunks_exact(2).enumerate() {
                            d += &format!("{}{} {} ", if i == 0 { "M" } else { "L" }, xy[0], xy[1]);
                        }
                        if tag == "polygon" {
                            d += "Z";
                        }
                        path_shape(&parse_d(&d), m, fill, rule)
                    }),
                    _ => None,
                };
                if let Some(mut s) = shape {
                    if let Some(id) = child.attribute("id") {
                        s.name = id.to_string();
                    }
                    out.push(s);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn basic(kind: ShapeKind, name: &str, x: f64, y: f64, w: f64, h: f64, rot: f64, radius: f64, fill: u32) -> Shape {
    Shape {
        id: 0,
        name: name.into(),
        kind,
        x: x as f32,
        y: y as f32,
        width: w as f32,
        height: h as f32,
        rotation: rot as f32,
        scale_x: 1.0,
        scale_y: 1.0,
        opacity: 1.0,
        corner_radius: radius as f32,
        fill,
        paths: Vec::new(),
        fill_rule: FillRule::NonZero,
    }
}

/// A path shape whose origin is the centre of its bounds.
fn path_shape(subpaths: &[(Vec<Seg>, bool)], m: M, fill: u32, rule: FillRule) -> Shape {
    let paths: Vec<SubPath> = subpaths.iter().map(|(s, c)| to_subpath(s, *c, m)).collect();
    let mut s = basic(ShapeKind::Path, "Path", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, fill);
    s.paths = paths;
    s.fill_rule = rule;
    s.recentre();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_paths_rects_and_rotation() {
        let svg = r##"<svg viewBox="0 0 100 100"><rect width="100" height="100" fill="black"/>
            <path d="M10 10 L30 10 C40 10 40 30 30 30 Z" fill="#F9A61A" fill-rule="evenodd"/>
            <rect x="50" y="50" width="10" height="20" rx="5" transform="rotate(90 50 50)" fill="white"/>
            <circle cx="20" cy="80" r="5" fill="none"/></svg>"##;
        let imp = import(svg).unwrap();
        assert_eq!(imp.shapes.len(), 3, "the unfilled circle is skipped");
        let p = &imp.shapes[1];
        assert_eq!(p.kind, ShapeKind::Path);
        assert_eq!(p.fill, 0xfff9a61a);
        assert_eq!(p.fill_rule, FillRule::EvenOdd);
        assert_eq!(p.paths[0].points.len(), 3, "the closing point merges into the first");
        assert!(p.paths[0].closed);
        let r = &imp.shapes[2];
        assert_eq!(r.kind, ShapeKind::Rectangle);
        assert!((r.rotation - 90.0).abs() < 1e-4);
        assert!((r.x - 40.0).abs() < 1e-4 && (r.y - 55.0).abs() < 1e-4, "rotated about its corner");
    }

    #[test]
    fn relative_and_arc_commands() {
        let segs = parse_d("m0 0 h10 v10 h-10 z M0 0 a5 5 0 0 1 10 0");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].0.len(), 4);
        let end = segs[1].0.last().unwrap().3;
        assert!((end.0 - 10.0).abs() < 1e-9 && end.1.abs() < 1e-9);
    }
}
