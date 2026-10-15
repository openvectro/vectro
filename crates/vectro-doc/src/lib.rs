//! Vectro's document: an artboard of shapes plus timelines of keyframes.
//! Plays back with the same rules as Rive's runtime and exports to `.riv`.

use serde::{Deserialize, Serialize};
use vectro_riv::{write, Builder, Schema, Value};

pub mod svg;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub version: u32,
    pub artboard: Artboard,
    pub shapes: Vec<Shape>,
    pub animations: Vec<Animation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Artboard {
    pub name: String,
    pub width: f32,
    pub height: f32,
    /// ARGB
    pub background: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShapeKind {
    Rectangle,
    Ellipse,
    Path,
    /// Draws nothing itself; its transform and opacity carry its children.
    Group,
}

/// A path point with its incoming and outgoing curve handles, relative to the shape's origin.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub in_x: f32,
    pub in_y: f32,
    pub out_x: f32,
    pub out_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubPath {
    pub closed: bool,
    pub points: Vec<Vertex>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// A shape's design values. Position is its centre; rotation is in degrees.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    pub id: u32,
    pub name: String,
    pub kind: ShapeKind,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub opacity: f32,
    pub corner_radius: f32,
    /// ARGB
    pub fill: u32,
    /// Outlines of a path shape; empty for rectangles and ellipses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<SubPath>,
    #[serde(default)]
    pub fill_rule: FillRule,
    /// The group this shape sits in; its x/y are then in the group's space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
}

impl Shape {
    /// Bounds of a path's points (handles included), relative to its origin.
    pub fn path_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut it = self.paths.iter().flat_map(|p| &p.points).flat_map(|v| [(v.x, v.y), (v.in_x, v.in_y), (v.out_x, v.out_y)]);
        let first = it.next()?;
        Some(it.fold((first.0, first.1, first.0, first.1), |(a, b, c, d), (x, y)| (a.min(x), b.min(y), c.max(x), d.max(y))))
    }

    /// Move a path's origin to the centre of its bounds without moving it on screen.
    pub fn recentre(&mut self) {
        let Some((l, t, r, b)) = self.path_bounds() else { return };
        let (cx, cy) = ((l + r) / 2.0, (t + b) / 2.0);
        for v in self.paths.iter_mut().flat_map(|p| p.points.iter_mut()) {
            for (x, y) in [(&mut v.x, &mut v.y), (&mut v.in_x, &mut v.in_y), (&mut v.out_x, &mut v.out_y)] {
                *x -= cx;
                *y -= cy;
            }
        }
        self.x += cx;
        self.y += cy;
        self.width = r - l;
        self.height = b - t;
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Prop {
    X,
    Y,
    Width,
    Height,
    Rotation,
    ScaleX,
    ScaleY,
    Opacity,
    CornerRadius,
    Fill,
}

impl Prop {
    pub const ALL: [Prop; 10] = [
        Prop::X,
        Prop::Y,
        Prop::Width,
        Prop::Height,
        Prop::Rotation,
        Prop::ScaleX,
        Prop::ScaleY,
        Prop::Opacity,
        Prop::CornerRadius,
        Prop::Fill,
    ];

    pub fn get(self, s: &Shape) -> f64 {
        match self {
            Prop::X => s.x as f64,
            Prop::Y => s.y as f64,
            Prop::Width => s.width as f64,
            Prop::Height => s.height as f64,
            Prop::Rotation => s.rotation as f64,
            Prop::ScaleX => s.scale_x as f64,
            Prop::ScaleY => s.scale_y as f64,
            Prop::Opacity => s.opacity as f64,
            Prop::CornerRadius => s.corner_radius as f64,
            Prop::Fill => s.fill as f64,
        }
    }

    pub fn set(self, s: &mut Shape, v: f64) {
        match self {
            Prop::X => s.x = v as f32,
            Prop::Y => s.y = v as f32,
            Prop::Width => s.width = v as f32,
            Prop::Height => s.height = v as f32,
            Prop::Rotation => s.rotation = v as f32,
            Prop::ScaleX => s.scale_x = v as f32,
            Prop::ScaleY => s.scale_y = v as f32,
            Prop::Opacity => s.opacity = v as f32,
            Prop::CornerRadius => s.corner_radius = v as f32,
            Prop::Fill => s.fill = v as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Ease {
    Hold,
    Linear,
    Cubic { x1: f32, y1: f32, x2: f32, y2: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub frame: u32,
    /// Numbers as they are; colours as an ARGB integer.
    pub value: f64,
    /// How the value travels from this key to the next one.
    pub ease: Ease,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub shape_id: u32,
    pub prop: Prop,
    pub keys: Vec<Key>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    OneShot,
    Loop,
    PingPong,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Animation {
    pub name: String,
    pub fps: u32,
    /// In frames.
    pub duration: u32,
    #[serde(rename = "loop")]
    pub loop_mode: LoopMode,
    pub tracks: Vec<Track>,
}

impl Animation {
    /// The frame shown `seconds` after the animation starts, honouring its loop mode.
    pub fn frame_at(&self, seconds: f64) -> f64 {
        let d = self.duration.max(1) as f64;
        let f = seconds * self.fps as f64;
        match self.loop_mode {
            LoopMode::OneShot => f.clamp(0.0, d),
            LoopMode::Loop => f.rem_euclid(d),
            LoopMode::PingPong => {
                let p = f.rem_euclid(2.0 * d);
                if p > d {
                    2.0 * d - p
                } else {
                    p
                }
            }
        }
    }
}

/// CSS-style cubic bezier easing: find t where x(t) = progress, return y(t).
pub fn cubic_ease(x1: f32, y1: f32, x2: f32, y2: f32, progress: f64) -> f64 {
    let (x1, y1, x2, y2) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64);
    let bez = |a: f64, b: f64, t: f64| 3.0 * a * (1.0 - t) * (1.0 - t) * t + 3.0 * b * (1.0 - t) * t * t + t * t * t;
    let slope = |a: f64, b: f64, t: f64| 3.0 * a * (1.0 - t) * (1.0 - t) + 6.0 * (b - a) * (1.0 - t) * t + 3.0 * (1.0 - b) * t * t;
    if progress <= 0.0 || progress >= 1.0 {
        return progress.clamp(0.0, 1.0);
    }
    let mut t = progress;
    for _ in 0..8 {
        let d = slope(x1, x2, t);
        if d.abs() < 1e-7 {
            break;
        }
        t -= (bez(x1, x2, t) - progress) / d;
    }
    if !(0.0..=1.0).contains(&t) || (bez(x1, x2, t) - progress).abs() > 1e-5 {
        let (mut lo, mut hi) = (0.0, 1.0);
        t = progress;
        for _ in 0..40 {
            let x = bez(x1, x2, t);
            if (x - progress).abs() < 1e-7 {
                break;
            }
            if x < progress {
                lo = t
            } else {
                hi = t
            }
            t = (lo + hi) / 2.0;
        }
    }
    bez(y1, y2, t)
}

fn lerp_colour(a: u32, b: u32, f: f64) -> u32 {
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as f64;
    let mut out = 0u32;
    for s in [24, 16, 8, 0] {
        let v = ch(a, s) + (ch(b, s) - ch(a, s)) * f;
        out |= (v.round().clamp(0.0, 255.0) as u32) << s;
    }
    out
}

impl Track {
    /// Value at `frame`, using the same rules as Rive: the earlier key's ease shapes each span.
    pub fn value_at(&self, frame: f64) -> Option<f64> {
        let keys = &self.keys;
        let first = keys.first()?;
        if frame <= first.frame as f64 {
            return Some(first.value);
        }
        for pair in keys.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            if frame < b.frame as f64 {
                let span = (b.frame - a.frame).max(1) as f64;
                let p = (frame - a.frame as f64) / span;
                let f = match a.ease {
                    Ease::Hold => 0.0,
                    Ease::Linear => p,
                    Ease::Cubic { x1, y1, x2, y2 } => cubic_ease(x1, y1, x2, y2, p),
                };
                return Some(if self.prop == Prop::Fill {
                    lerp_colour(a.value as u32, b.value as u32, f) as f64
                } else {
                    a.value + (b.value - a.value) * f
                });
            }
        }
        Some(keys.last().unwrap().value)
    }
}

impl Document {
    /// Add an SVG's shapes, scaled to fit inside the artboard with a margin and centred.
    pub fn import_svg(&mut self, svg: &str) -> Result<Vec<u32>, String> {
        let imp = svg::import(svg)?;
        let ab = &self.artboard;
        let k = (ab.width * 0.8 / imp.width).min(ab.height * 0.8 / imp.height).min(1.0);
        let (ox, oy) = ((ab.width - imp.width * k) / 2.0, (ab.height - imp.height * k) / 2.0);
        let mut next = self.shapes.iter().map(|s| s.id).max().unwrap_or(0);
        let mut ids = Vec::new();
        for mut s in imp.shapes {
            next += 1;
            s.id = next;
            s.x = s.x * k + ox;
            s.y = s.y * k + oy;
            s.width *= k;
            s.height *= k;
            s.corner_radius *= k;
            for v in s.paths.iter_mut().flat_map(|p| p.points.iter_mut()) {
                for c in [&mut v.x, &mut v.y, &mut v.in_x, &mut v.in_y, &mut v.out_x, &mut v.out_y] {
                    *c *= k;
                }
            }
            ids.push(s.id);
            self.shapes.push(s);
        }
        Ok(ids)
    }

    pub fn new() -> Self {
        Document {
            version: 1,
            artboard: Artboard { name: "Artboard".into(), width: 500.0, height: 500.0, background: 0xff1e1e24 },
            shapes: Vec::new(),
            animations: vec![Animation {
                name: "Animation 1".into(),
                fps: 60,
                duration: 60,
                loop_mode: LoopMode::Loop,
                tracks: Vec::new(),
            }],
        }
    }

    /// Shapes as they look at `frame` of animation `anim` (design values when `anim` is None).
    pub fn evaluate(&self, anim: Option<usize>, frame: f64) -> Vec<Shape> {
        let mut shapes = self.shapes.clone();
        if let Some(a) = anim.and_then(|i| self.animations.get(i)) {
            for t in &a.tracks {
                if let (Some(s), Some(v)) = (shapes.iter_mut().find(|s| s.id == t.shape_id), t.value_at(frame)) {
                    t.prop.set(s, v);
                }
            }
        }
        shapes
    }

    pub fn export_riv(&self) -> Vec<u8> {
        write(&export(self, &Schema::bundled()))
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Artboard component indices for one shape.
struct Placed {
    shape: u64,
    path: Option<u64>,
    colour: Option<u64>,
}

fn export(doc: &Document, schema: &Schema) -> vectro_riv::RivFile {
    use Value::{Color, Double, String as Str, Uint};
    let mut b = Builder::new(schema);
    // Index 0 is the artboard; every component after it counts up from 1.
    let mut next = 0u64;
    let mut comp = |_: usize| {
        next += 1;
        next
    };

    b.add("Backboard", &[]);
    let ab = &doc.artboard;
    b.add("Artboard", &[
        ("name", Str(ab.name.clone())),
        ("width", Double(ab.width)),
        ("height", Double(ab.height)),
    ]);
    let bg_fill = comp(b.add("Fill", &[("parentId", Uint(0))]));
    comp(b.add("SolidColor", &[("parentId", Uint(bg_fill)), ("colorValue", Color(ab.background))]));

    // Rive draws earlier components on top, so each container's children go front-most first
    // (last in the editor's list first), depth first so a group comes before what it holds.
    let mut placed = Vec::new();
    let mut stack: Vec<(u32, u64)> = Vec::new(); // (shape id, parent component), popped in order
    let roots = doc.shapes.iter().filter(|s| s.parent.map_or(true, |p| !doc.shapes.iter().any(|o| o.id == p && o.kind == ShapeKind::Group)));
    for s in roots.collect::<Vec<_>>() {
        stack.push((s.id, 0));
    }
    while let Some((id, parent)) = stack.pop() {
        let Some(s) = doc.shapes.iter().find(|s| s.id == id) else { continue };
        if s.kind == ShapeKind::Group {
            let node = comp(b.add("Node", &[
                ("name", Str(s.name.clone())),
                ("parentId", Uint(parent)),
                ("x", Double(s.x)),
                ("y", Double(s.y)),
                ("rotation", Double(s.rotation.to_radians())),
                ("scaleX", Double(s.scale_x)),
                ("scaleY", Double(s.scale_y)),
                ("opacity", Double(s.opacity)),
            ]));
            placed.push((s.id, s.kind, Placed { shape: node, path: None, colour: None }));
            for c in doc.shapes.iter().filter(|c| c.parent == Some(s.id)) {
                stack.push((c.id, node));
            }
            continue;
        }
        let shape = comp(b.add("Shape", &[
            ("name", Str(s.name.clone())),
            ("parentId", Uint(parent)),
            ("x", Double(s.x)),
            ("y", Double(s.y)),
            ("rotation", Double(s.rotation.to_radians())),
            ("scaleX", Double(s.scale_x)),
            ("scaleY", Double(s.scale_y)),
            ("opacity", Double(s.opacity)),
        ]));
        let path = match s.kind {
            ShapeKind::Rectangle => {
                let r = s.corner_radius;
                comp(b.add("Rectangle", &[
                    ("parentId", Uint(shape)),
                    ("width", Double(s.width)),
                    ("height", Double(s.height)),
                    ("cornerRadiusTL", Double(r)),
                    ("cornerRadiusTR", Double(r)),
                    ("cornerRadiusBL", Double(r)),
                    ("cornerRadiusBR", Double(r)),
                ]))
            }
            ShapeKind::Ellipse => comp(b.add("Ellipse", &[
                ("parentId", Uint(shape)),
                ("width", Double(s.width)),
                ("height", Double(s.height)),
            ])),
            ShapeKind::Group => unreachable!("groups are placed above"),
            ShapeKind::Path => {
                let mut first = None;
                for sub in &s.paths {
                    let path = comp(b.add("PointsPath", &[("parentId", Uint(shape)), ("isClosed", Value::Bool(sub.closed))]));
                    first.get_or_insert(path);
                    for v in &sub.points {
                        let (ix, iy, ox, oy) = (v.in_x - v.x, v.in_y - v.y, v.out_x - v.x, v.out_y - v.y);
                        if ix.hypot(iy) < 1e-4 && ox.hypot(oy) < 1e-4 {
                            comp(b.add("StraightVertex", &[("parentId", Uint(path)), ("x", Double(v.x)), ("y", Double(v.y))]));
                        } else {
                            comp(b.add("CubicDetachedVertex", &[
                                ("parentId", Uint(path)),
                                ("x", Double(v.x)),
                                ("y", Double(v.y)),
                                ("inRotation", Double(iy.atan2(ix))),
                                ("inDistance", Double(ix.hypot(iy))),
                                ("outRotation", Double(oy.atan2(ox))),
                                ("outDistance", Double(ox.hypot(oy))),
                            ]));
                        }
                    }
                }
                first.unwrap_or(shape)
            }
        };
        let fill_rule = match s.fill_rule {
            FillRule::NonZero => 0,
            FillRule::EvenOdd => 1,
        };
        let fill = comp(b.add("Fill", &[("parentId", Uint(shape)), ("fillRule", Uint(fill_rule))]));
        let colour = comp(b.add("SolidColor", &[("parentId", Uint(fill)), ("colorValue", Color(s.fill))]));
        placed.push((s.id, s.kind, Placed { shape, path: Some(path), colour: Some(colour) }));
    }

    // Each distinct cubic ease becomes one interpolator component that keyframes point at.
    let mut eases: Vec<(Ease, u64)> = Vec::new();
    for a in &doc.animations {
        for t in &a.tracks {
            for k in &t.keys {
                if let Ease::Cubic { x1, y1, x2, y2 } = k.ease {
                    if !eases.iter().any(|(e, _)| *e == k.ease) {
                        let at = comp(b.add("CubicEaseInterpolator", &[
                            ("x1", Double(x1)),
                            ("y1", Double(y1)),
                            ("x2", Double(x2)),
                            ("y2", Double(y2)),
                        ]));
                        eases.push((k.ease, at));
                    }
                }
            }
        }
    }

    for a in &doc.animations {
        b.add("LinearAnimation", &[
            ("name", Str(a.name.clone())),
            ("fps", Uint(a.fps as u64)),
            ("duration", Uint(a.duration as u64)),
            ("loopValue", Uint(match a.loop_mode {
                LoopMode::OneShot => 0,
                LoopMode::Loop => 1,
                LoopMode::PingPong => 2,
            })),
        ]);
        // Group tracks by the component they animate: one KeyedObject each.
        let mut targets: Vec<(u64, Vec<(&str, &str, &Track)>)> = Vec::new();
        for t in &a.tracks {
            if t.keys.is_empty() {
                continue;
            }
            let Some((_, kind, p)) = placed.iter().find(|(id, _, _)| *id == t.shape_id) else { continue };
            let path_type = match kind {
                ShapeKind::Rectangle => "Rectangle",
                ShapeKind::Ellipse => "Ellipse",
                ShapeKind::Path | ShapeKind::Group => "",
            };
            let (path, colour) = (p.path.unwrap_or(p.shape), p.colour.unwrap_or(p.shape));
            let entries: Vec<(u64, &str, &str)> = match t.prop {
                Prop::Width | Prop::Height | Prop::CornerRadius | Prop::Fill if *kind == ShapeKind::Group => vec![],
                Prop::X | Prop::Y | Prop::Rotation | Prop::ScaleX | Prop::ScaleY | Prop::Opacity if *kind == ShapeKind::Group => {
                    let name = match t.prop {
                        Prop::X => "x",
                        Prop::Y => "y",
                        Prop::Rotation => "rotation",
                        Prop::ScaleX => "scaleX",
                        Prop::ScaleY => "scaleY",
                        _ => "opacity",
                    };
                    vec![(p.shape, "Node", name)]
                }
                Prop::X => vec![(p.shape, "Shape", "x")],
                Prop::Y => vec![(p.shape, "Shape", "y")],
                Prop::Rotation => vec![(p.shape, "Shape", "rotation")],
                Prop::ScaleX => vec![(p.shape, "Shape", "scaleX")],
                Prop::ScaleY => vec![(p.shape, "Shape", "scaleY")],
                Prop::Opacity => vec![(p.shape, "Shape", "opacity")],
                Prop::Width | Prop::Height if *kind == ShapeKind::Path => vec![],
                Prop::Width => vec![(path, path_type, "width")],
                Prop::Height => vec![(path, path_type, "height")],
                Prop::CornerRadius if *kind == ShapeKind::Rectangle => ["cornerRadiusTL", "cornerRadiusTR", "cornerRadiusBL", "cornerRadiusBR"]
                    .into_iter()
                    .map(|n| (path, "Rectangle", n))
                    .collect(),
                Prop::CornerRadius => vec![],
                Prop::Fill => vec![(colour, "SolidColor", "colorValue")],
            };
            for (object, owner, name) in entries {
                match targets.iter_mut().find(|(o, _)| *o == object) {
                    Some((_, list)) => list.push((owner, name, t)),
                    None => targets.push((object, vec![(owner, name, t)])),
                }
            }
        }
        for (object, list) in targets {
            b.add("KeyedObject", &[("objectId", Uint(object))]);
            for (owner, name, t) in list {
                let key = b.schema().property_key(owner, name).expect("known property");
                b.add("KeyedProperty", &[("propertyKey", Uint(key))]);
                let mut keys = t.keys.clone();
                keys.sort_by_key(|k| k.frame);
                for k in keys {
                    let (kind, interpolator) = match k.ease {
                        Ease::Hold => (0, None),
                        Ease::Linear => (1, None),
                        Ease::Cubic { .. } => (2, eases.iter().find(|(e, _)| *e == k.ease).map(|(_, i)| *i)),
                    };
                    let mut props = vec![("frame", Uint(k.frame as u64)), ("interpolationType", Uint(kind))];
                    if let Some(i) = interpolator {
                        props.push(("interpolatorId", Uint(i)));
                    }
                    if t.prop == Prop::Fill {
                        props.push(("value", Color(k.value as u32)));
                        b.add("KeyFrameColor", &props);
                    } else {
                        let v = if t.prop == Prop::Rotation { (k.value as f32).to_radians() } else { k.value as f32 };
                        props.push(("value", Double(v)));
                        b.add("KeyFrameDouble", &props);
                    }
                }
            }
        }
    }

    // A default state machine that plays the first animation, for apps that drive state machines.
    if !doc.animations.is_empty() {
        b.add("StateMachine", &[("name", Str("State Machine 1".into()))]);
        b.add("StateMachineLayer", &[("name", Str("Layer 1".into()))]);
        b.add("EntryState", &[]);
        b.add("StateTransition", &[("stateToId", Uint(3))]);
        b.add("AnyState", &[]);
        b.add("ExitState", &[]);
        b.add("AnimationState", &[("animationId", Uint(0))]);
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Document {
        let mut d = Document::new();
        d.shapes.push(Shape {
            id: 1,
            name: "ball".into(),
            kind: ShapeKind::Ellipse,
            x: 100.0,
            y: 250.0,
            width: 80.0,
            height: 80.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
            corner_radius: 0.0,
            fill: 0xffff6b6b,
            paths: Vec::new(),
            fill_rule: FillRule::NonZero,
            parent: None,
        });
        d.animations[0].tracks.push(Track {
            shape_id: 1,
            prop: Prop::X,
            keys: vec![
                Key { frame: 0, value: 100.0, ease: Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 } },
                Key { frame: 60, value: 400.0, ease: Ease::Linear },
            ],
        });
        d.animations[0].tracks.push(Track {
            shape_id: 1,
            prop: Prop::Fill,
            keys: vec![
                Key { frame: 0, value: 0xffff0000u32 as f64, ease: Ease::Linear },
                Key { frame: 60, value: 0xff0000ffu32 as f64, ease: Ease::Linear },
            ],
        });
        d
    }

    #[test]
    fn playback_follows_keys_and_easing() {
        let d = demo();
        let at = |f| d.evaluate(Some(0), f)[0].clone();
        assert_eq!(at(0.0).x, 100.0);
        assert_eq!(at(60.0).x, 400.0);
        assert!((at(30.0).x - 250.0).abs() < 0.5, "ease-in-out is symmetric at the midpoint");
        assert!(at(10.0).x < 100.0 + 300.0 * 10.0 / 60.0, "ease-in starts slower than linear");
        assert_eq!(at(30.0).fill, 0xff800080, "colours blend per channel");
    }

    #[test]
    fn loop_modes() {
        let mut a = Document::new().animations.remove(0);
        a.duration = 60;
        assert_eq!(a.frame_at(1.5), 30.0);
        a.loop_mode = LoopMode::PingPong;
        assert_eq!(a.frame_at(1.5), 30.0);
        assert_eq!(a.frame_at(1.25), 45.0);
        a.loop_mode = LoopMode::OneShot;
        assert_eq!(a.frame_at(5.0), 60.0);
    }

    #[test]
    fn export_reads_back() {
        let bytes = demo().export_riv();
        let schema = Schema::bundled();
        let file = vectro_riv::read(&bytes, &schema).expect("valid .riv");
        let names: Vec<_> = file.objects.iter().filter_map(|o| o.type_name.clone()).collect();
        for want in ["Artboard", "Shape", "Ellipse", "CubicEaseInterpolator", "LinearAnimation", "KeyFrameColor", "StateMachine"] {
            assert!(names.iter().any(|n| n == want), "missing {want}");
        }
        assert_eq!(vectro_riv::write(&file), bytes);
    }

    #[test]
    fn svg_paths_export_and_read_back() {
        let imp = svg::import(r##"<svg viewBox="0 0 100 100"><path d="M10 10 L90 10 C90 60 60 90 10 90 Z" fill="#F9A61A" fill-rule="evenodd"/></svg>"##).unwrap();
        let mut d = Document::new();
        d.shapes = imp.shapes;
        let bytes = d.export_riv();
        let file = vectro_riv::read(&bytes, &Schema::bundled()).expect("valid .riv");
        let count = |n: &str| file.objects.iter().filter(|o| o.type_name.as_deref() == Some(n)).count();
        assert_eq!(count("PointsPath"), 1);
        assert_eq!(count("StraightVertex") + count("CubicDetachedVertex"), 3);
        assert_eq!(count("CubicDetachedVertex"), 2, "the corners touching the curve carry handles");
    }

    #[test]
    fn groups_export_as_nodes_holding_their_children() {
        let mut d = demo();
        let mut g = d.shapes[0].clone();
        g.id = 9;
        g.kind = ShapeKind::Group;
        g.name = "group".into();
        d.shapes.insert(0, g);
        d.shapes[1].parent = Some(9);
        d.animations[0].tracks.push(Track { shape_id: 9, prop: Prop::Rotation, keys: vec![Key { frame: 0, value: 0.0, ease: Ease::Linear }, Key { frame: 60, value: 90.0, ease: Ease::Linear }] });
        let bytes = d.export_riv();
        let file = vectro_riv::read(&bytes, &Schema::bundled()).expect("valid .riv");
        let node = file.objects.iter().position(|o| o.type_name.as_deref() == Some("Node")).expect("a Node");
        let shape = file.objects.iter().find(|o| o.type_name.as_deref() == Some("Shape")).unwrap();
        let parent = shape.properties.iter().find(|p| p.name.as_deref() == Some("parentId")).unwrap();
        // Component index = file position - 1 (the backboard comes first).
        assert_eq!(parent.value, Value::Uint(node as u64 - 1));
    }

    #[test]
    fn json_round_trip() {
        let d = demo();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<Document>(&json).unwrap(), d);
    }
}
