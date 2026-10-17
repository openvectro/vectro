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
    /// Names of events the file can report to the app (see `Action::FireEvent`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_machines: Vec<StateMachine>,
}

// ----------------------------------------------------------------- state machines

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InputKind {
    Bool,
    Number,
    Trigger,
}

/// Something the app (or a listener) sets to drive the machine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Input {
    pub name: String,
    pub kind: InputKind,
    /// Starting value: 0 or 1 for a bool, any number for a number, unused for a trigger.
    #[serde(default)]
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StateKind {
    Entry,
    Any,
    Exit,
    /// Plays one animation (index into `Document::animations`).
    Animation { animation: usize, #[serde(default = "one")] speed: f32 },
    /// Mixes animations by a number input: each child is (animation, the input value it stands for).
    Blend1D { input: usize, children: Vec<(usize, f32)> },
}

fn one() -> f32 {
    1.0
}

fn half() -> f32 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub name: String,
    pub kind: StateKind,
    /// Pick among this state's valid transitions at random, by `random_weight`.
    #[serde(default)]
    pub random: bool,
    /// Where the editor draws this state in the graph; not part of the exported file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Op {
    Equal = 0,
    NotEqual = 1,
    LessOrEqual = 2,
    GreaterOrEqual = 3,
    Less = 4,
    Greater = 5,
}

/// A test on one input. For a bool, `value` 1 means "is true"; for a trigger only `input` matters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    pub input: usize,
    #[serde(default = "equal")]
    pub op: Op,
    #[serde(default)]
    pub value: f64,
}

fn equal() -> Op {
    Op::Equal
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Indices into the layer's `states`.
    pub from: usize,
    pub to: usize,
    /// Blend length between the two states.
    #[serde(default)]
    pub duration_ms: u32,
    /// Only after the source state has played this long.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_after_ms: Option<u32>,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default = "one_u32")]
    pub random_weight: u32,
}

fn one_u32() -> u32 {
    1
}

/// States and transitions that run on their own; every layer of a machine runs at once.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub name: String,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Pointer {
    Enter = 0,
    Exit = 1,
    Down = 2,
    Up = 3,
    Move = 4,
    Click = 6,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Action {
    SetBool { input: usize, value: bool },
    ToggleBool { input: usize },
    SetNumber { input: usize, value: f64 },
    FireTrigger { input: usize },
    /// Index into `Document::events`.
    FireEvent { event: usize },
}

/// Pointer activity on a shape (or group) that changes inputs, with no app code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Listener {
    pub target: u32,
    pub event: Pointer,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StateMachine {
    pub name: String,
    pub inputs: Vec<Input>,
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub listeners: Vec<Listener>,
}

impl Layer {
    /// A layer with the three states every Rive layer starts with: Entry (0), Any (1), Exit (2).
    pub fn new(name: &str) -> Self {
        let std = |n: &str, kind| State { name: n.into(), kind, random: false, ui: None };
        Layer {
            name: name.into(),
            states: vec![std("Entry", StateKind::Entry), std("Any", StateKind::Any), std("Exit", StateKind::Exit)],
            transitions: Vec::new(),
        }
    }

    /// Add a state that plays `animation`; returns its index.
    pub fn animation(&mut self, name: &str, animation: usize) -> usize {
        self.states.push(State { name: name.into(), kind: StateKind::Animation { animation, speed: 1.0 }, random: false, ui: None });
        self.states.len() - 1
    }

    pub fn blend(&mut self, name: &str, input: usize, children: Vec<(usize, f32)>) -> usize {
        self.states.push(State { name: name.into(), kind: StateKind::Blend1D { input, children }, random: false, ui: None });
        self.states.len() - 1
    }

    pub fn go(&mut self, from: usize, to: usize) -> &mut Transition {
        self.transitions.push(Transition { from, to, duration_ms: 0, exit_after_ms: None, conditions: Vec::new(), random_weight: 1 });
        self.transitions.last_mut().unwrap()
    }
}

impl Transition {
    pub fn blend_ms(&mut self, ms: u32) -> &mut Self {
        self.duration_ms = ms;
        self
    }

    pub fn after_ms(&mut self, ms: u32) -> &mut Self {
        self.exit_after_ms = Some(ms);
        self
    }

    pub fn when(&mut self, input: usize, op: Op, value: f64) -> &mut Self {
        self.conditions.push(Condition { input, op, value });
        self
    }

    pub fn when_true(&mut self, input: usize) -> &mut Self {
        self.when(input, Op::Equal, 1.0)
    }

    pub fn when_false(&mut self, input: usize) -> &mut Self {
        self.when(input, Op::Equal, 0.0)
    }

    pub fn on_trigger(&mut self, input: usize) -> &mut Self {
        self.when(input, Op::Equal, 0.0)
    }

    pub fn weight(&mut self, w: u32) -> &mut Self {
        self.random_weight = w;
        self
    }
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
    /// Where a rectangle's or ellipse's origin sits in its box, 0..1 each way (0.5 is the centre).
    #[serde(default = "half")]
    pub origin_x: f32,
    #[serde(default = "half")]
    pub origin_y: f32,
    /// Replaces the flat `fill` when set. Coordinates are in the shape's own space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Gradient>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<Stroke>,
    /// Rive's feather: softens the fill into a glow or shadow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feather: Option<Feather>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GradientKind {
    Linear,
    Radial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stop {
    pub position: f32,
    /// ARGB
    pub color: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Gradient {
    pub kind: GradientKind,
    pub start: (f32, f32),
    /// For a radial gradient, a point on the outer circle.
    pub end: (f32, f32),
    pub stops: Vec<Stop>,
}

impl Gradient {
    pub fn linear(x0: f32, y0: f32, x1: f32, y1: f32, stops: &[(f32, u32)]) -> Self {
        Gradient { kind: GradientKind::Linear, start: (x0, y0), end: (x1, y1), stops: stops.iter().map(|&(position, color)| Stop { position, color }).collect() }
    }

    pub fn radial(cx: f32, cy: f32, radius: f32, stops: &[(f32, u32)]) -> Self {
        Gradient { kind: GradientKind::Radial, start: (cx, cy), end: (cx + radius, cy), stops: stops.iter().map(|&(position, color)| Stop { position, color }).collect() }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stroke {
    /// ARGB
    pub color: u32,
    pub thickness: f32,
    /// 0 butt, 1 round, 2 square
    #[serde(default)]
    pub cap: u8,
    /// 0 miter, 1 round, 2 bevel
    #[serde(default)]
    pub join: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Feather {
    pub strength: f32,
    #[serde(default)]
    pub offset: (f32, f32),
    #[serde(default)]
    pub inner: bool,
}

impl Shape {
    /// A rectangle or ellipse at centre (x, y). `fill` is ARGB.
    pub fn new(id: u32, name: &str, kind: ShapeKind, x: f32, y: f32, width: f32, height: f32, fill: u32) -> Self {
        Shape {
            id,
            name: name.into(),
            kind,
            x,
            y,
            width,
            height,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
            corner_radius: 0.0,
            fill,
            paths: Vec::new(),
            fill_rule: FillRule::NonZero,
            parent: None,
            origin_x: 0.5,
            origin_y: 0.5,
            gradient: None,
            stroke: None,
            feather: None,
        }
    }

    pub fn gradient(mut self, g: Gradient) -> Self {
        self.gradient = Some(g);
        self
    }

    pub fn stroke(mut self, color: u32, thickness: f32) -> Self {
        self.stroke = Some(Stroke { color, thickness, cap: 1, join: 1 });
        self
    }

    pub fn feather(mut self, strength: f32, dx: f32, dy: f32) -> Self {
        self.feather = Some(Feather { strength, offset: (dx, dy), inner: false });
        self
    }

    pub fn inner_shadow(mut self, strength: f32, dx: f32, dy: f32) -> Self {
        self.feather = Some(Feather { strength, offset: (dx, dy), inner: true });
        self
    }

    pub fn in_group(mut self, parent: u32) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn radius(mut self, r: f32) -> Self {
        self.corner_radius = r;
        self
    }

    pub fn opacity(mut self, o: f32) -> Self {
        self.opacity = o;
        self
    }

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
    /// The last stop of a gradient fill (Fill is the first stop, or the flat colour).
    FillEnd,
    /// Point animation, for tracks with a `point`: its position, and its curve handles as an
    /// angle (degrees) and a length, the way Rive stores them.
    PointX,
    PointY,
    InAngle,
    InLength,
    OutAngle,
    OutLength,
}

impl Prop {
    pub fn is_point(self) -> bool {
        matches!(self, Prop::PointX | Prop::PointY | Prop::InAngle | Prop::InLength | Prop::OutAngle | Prop::OutLength)
    }

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
            Prop::Fill => s.gradient.as_ref().and_then(|g| g.stops.first()).map_or(s.fill, |st| st.color) as f64,
            Prop::FillEnd => s.gradient.as_ref().and_then(|g| g.stops.last()).map_or(s.fill, |st| st.color) as f64,
            _ => 0.0,
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
            Prop::Fill => match s.gradient.as_mut().and_then(|g| g.stops.first_mut()) {
                Some(st) => st.color = v as u32,
                None => s.fill = v as u32,
            },
            Prop::FillEnd => match s.gradient.as_mut().and_then(|g| g.stops.last_mut()) {
                Some(st) => st.color = v as u32,
                None => s.fill = v as u32,
            },
            _ => {}
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

/// One point of a path shape: outline `path`, point `index`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct PointRef {
    pub path: u32,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub shape_id: u32,
    pub prop: Prop,
    /// Set for point animation (Prop::PointX and friends).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<PointRef>,
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
    pub fn new(name: &str, fps: u32, duration: u32, loop_mode: LoopMode) -> Self {
        Animation { name: name.into(), fps, duration, loop_mode, tracks: Vec::new() }
    }

    /// Key `prop` of a shape: `keys` are (frame, value, ease out of that key).
    pub fn key(&mut self, shape_id: u32, prop: Prop, keys: &[(u32, f64, Ease)]) -> &mut Self {
        self.tracks.push(Track {
            shape_id,
            prop,
            point: None,
            keys: keys.iter().map(|&(frame, value, ease)| Key { frame, value, ease }).collect(),
        });
        self
    }

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
                return Some(if matches!(self.prop, Prop::Fill | Prop::FillEnd) {
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
            events: Vec::new(),
            state_machines: Vec::new(),
        }
    }

    /// Shapes as they look at `frame` of animation `anim` (design values when `anim` is None).
    pub fn evaluate(&self, anim: Option<usize>, frame: f64) -> Vec<Shape> {
        let mut shapes = self.shapes.clone();
        if let Some(a) = anim.and_then(|i| self.animations.get(i)) {
            for t in a.tracks.iter().filter(|t| t.point.is_none()) {
                if let (Some(s), Some(v)) = (shapes.iter_mut().find(|s| s.id == t.shape_id), t.value_at(frame)) {
                    t.prop.set(s, v);
                }
            }
            // Points: work in Rive's terms (position plus polar handles) so playback matches it.
            for t in a.tracks.iter().filter(|t| t.point.is_some()) {
                let pr = t.point.unwrap();
                let (Some(s), Some(v)) = (shapes.iter_mut().find(|s| s.id == t.shape_id), t.value_at(frame)) else { continue };
                let Some(p) = s.paths.get_mut(pr.path as usize).and_then(|sub| sub.points.get_mut(pr.index as usize)) else { continue };
                let polar = |hx: f32, hy: f32, x: f32, y: f32| ((hy - y).atan2(hx - x), (hx - x).hypot(hy - y));
                let (mut ia, mut il) = polar(p.in_x, p.in_y, p.x, p.y);
                let (mut oa, mut ol) = polar(p.out_x, p.out_y, p.x, p.y);
                let v = v as f32;
                match t.prop {
                    Prop::PointX => p.x = v,
                    Prop::PointY => p.y = v,
                    Prop::InAngle => ia = v.to_radians(),
                    Prop::InLength => il = v,
                    Prop::OutAngle => oa = v.to_radians(),
                    Prop::OutLength => ol = v,
                    _ => {}
                }
                p.in_x = p.x + il * ia.cos();
                p.in_y = p.y + il * ia.sin();
                p.out_x = p.x + ol * oa.cos();
                p.out_y = p.y + ol * oa.sin();
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
    /// The component holding the fill colour, and its type (SolidColor or GradientStop).
    colour: Option<(u64, &'static str)>,
    colour_end: Option<(u64, &'static str)>,
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
    let animated_handles: std::collections::HashSet<(u32, u32, u32)> = doc
        .animations
        .iter()
        .flat_map(|a| &a.tracks)
        .filter(|t| matches!(t.prop, Prop::InAngle | Prop::InLength | Prop::OutAngle | Prop::OutLength))
        .filter_map(|t| t.point.map(|p| (t.shape_id, p.path, p.index)))
        .collect();
    let mut vertices: std::collections::HashMap<(u32, u32, u32), (u64, bool)> = std::collections::HashMap::new();
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
            placed.push((s.id, s.kind, Placed { shape: node, path: None, colour: None, colour_end: None }));
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
                    ("originX", Double(s.origin_x)),
                    ("originY", Double(s.origin_y)),
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
                ("originX", Double(s.origin_x)),
                ("originY", Double(s.origin_y)),
            ])),
            ShapeKind::Group => unreachable!("groups are placed above"),
            ShapeKind::Path => {
                let mut first = None;
                for (pi, sub) in s.paths.iter().enumerate() {
                    let path = comp(b.add("PointsPath", &[("parentId", Uint(shape)), ("isClosed", Value::Bool(sub.closed))]));
                    first.get_or_insert(path);
                    for (vi, v) in sub.points.iter().enumerate() {
                        let (ix, iy, ox, oy) = (v.in_x - v.x, v.in_y - v.y, v.out_x - v.x, v.out_y - v.y);
                        let key = (s.id, pi as u32, vi as u32);
                        if ix.hypot(iy) < 1e-4 && ox.hypot(oy) < 1e-4 && !animated_handles.contains(&key) {
                            let c = comp(b.add("StraightVertex", &[("parentId", Uint(path)), ("x", Double(v.x)), ("y", Double(v.y))]));
                            vertices.insert(key, (c, false));
                        } else {
                            let c = comp(b.add("CubicDetachedVertex", &[
                                ("parentId", Uint(path)),
                                ("x", Double(v.x)),
                                ("y", Double(v.y)),
                                ("inRotation", Double(iy.atan2(ix))),
                                ("inDistance", Double(ix.hypot(iy))),
                                ("outRotation", Double(oy.atan2(ox))),
                                ("outDistance", Double(ox.hypot(oy))),
                            ]));
                            vertices.insert(key, (c, true));
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
        let (colour, colour_end) = match &s.gradient {
            None => {
                let c = comp(b.add("SolidColor", &[("parentId", Uint(fill)), ("colorValue", Color(s.fill))]));
                ((c, "SolidColor"), (c, "SolidColor"))
            }
            Some(g) => {
                let kind = if g.kind == GradientKind::Linear { "LinearGradient" } else { "RadialGradient" };
                let gid = comp(b.add(kind, &[
                    ("parentId", Uint(fill)),
                    ("startX", Double(g.start.0)),
                    ("startY", Double(g.start.1)),
                    ("endX", Double(g.end.0)),
                    ("endY", Double(g.end.1)),
                ]));
                let stops: Vec<u64> = g
                    .stops
                    .iter()
                    .map(|st| comp(b.add("GradientStop", &[("parentId", Uint(gid)), ("colorValue", Color(st.color)), ("position", Double(st.position))])))
                    .collect();
                let first = *stops.first().unwrap_or(&gid);
                ((first, "GradientStop"), (*stops.last().unwrap_or(&first), "GradientStop"))
            }
        };
        if let Some(f) = &s.feather {
            comp(b.add("Feather", &[
                ("parentId", Uint(fill)),
                ("strength", Double(f.strength)),
                ("offsetX", Double(f.offset.0)),
                ("offsetY", Double(f.offset.1)),
                ("inner", Value::Bool(f.inner)),
            ]));
        }
        if let Some(st) = &s.stroke {
            let sid = comp(b.add("Stroke", &[
                ("parentId", Uint(shape)),
                ("thickness", Double(st.thickness)),
                ("cap", Uint(st.cap as u64)),
                ("join", Uint(st.join as u64)),
            ]));
            comp(b.add("SolidColor", &[("parentId", Uint(sid)), ("colorValue", Color(st.color))]));
        }
        placed.push((s.id, s.kind, Placed { shape, path: Some(path), colour: Some(colour), colour_end: Some(colour_end) }));
    }

    // Events are artboard components the app can listen for.
    let event_ids: Vec<u64> = doc
        .events
        .iter()
        .map(|name| comp(b.add("Event", &[("name", Str(name.clone())), ("parentId", Uint(0))])))
        .collect();

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
            let path = p.path.unwrap_or(p.shape);
            let (colour, colour_owner) = p.colour.unwrap_or((p.shape, "SolidColor"));
            let (colour_end, colour_end_owner) = p.colour_end.unwrap_or((p.shape, "SolidColor"));
            let vertex = t.point.and_then(|pr| vertices.get(&(t.shape_id, pr.path, pr.index)).copied());
            let entries: Vec<(u64, &str, &str)> = match t.prop {
                _ if t.prop.is_point() => match (vertex, t.prop) {
                    (None, _) => vec![],
                    (Some((v, _)), Prop::PointX) => vec![(v, "CubicDetachedVertex", "x")],
                    (Some((v, _)), Prop::PointY) => vec![(v, "CubicDetachedVertex", "y")],
                    (Some((v, true)), Prop::InAngle) => vec![(v, "CubicDetachedVertex", "inRotation")],
                    (Some((v, true)), Prop::InLength) => vec![(v, "CubicDetachedVertex", "inDistance")],
                    (Some((v, true)), Prop::OutAngle) => vec![(v, "CubicDetachedVertex", "outRotation")],
                    (Some((v, true)), Prop::OutLength) => vec![(v, "CubicDetachedVertex", "outDistance")],
                    _ => vec![],
                },
                Prop::Width | Prop::Height | Prop::CornerRadius | Prop::Fill | Prop::FillEnd if *kind == ShapeKind::Group => vec![],
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
                Prop::Fill => vec![(colour, colour_owner, "colorValue")],
                Prop::FillEnd => vec![(colour_end, colour_end_owner, "colorValue")],
                _ => vec![],
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
                    if matches!(t.prop, Prop::Fill | Prop::FillEnd) {
                        props.push(("value", Color(k.value as u32)));
                        b.add("KeyFrameColor", &props);
                    } else {
                        let v = if matches!(t.prop, Prop::Rotation | Prop::InAngle | Prop::OutAngle) {
                            (k.value as f32).to_radians()
                        } else {
                            k.value as f32
                        };
                        props.push(("value", Double(v)));
                        b.add("KeyFrameDouble", &props);
                    }
                }
            }
        }
    }

    for sm in &doc.state_machines {
        export_state_machine(&mut b, sm, &placed, &event_ids);
    }
    // A default state machine that plays the first animation, for apps that drive state machines.
    if doc.state_machines.is_empty() && !doc.animations.is_empty() {
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

fn export_state_machine(b: &mut Builder, sm: &StateMachine, placed: &[(u32, ShapeKind, Placed)], events: &[u64]) {
    use Value::{Double, String as Str, Uint};
    b.add("StateMachine", &[("name", Str(sm.name.clone()))]);
    for i in &sm.inputs {
        let name = ("name", Str(i.name.clone()));
        match i.kind {
            InputKind::Bool => b.add("StateMachineBool", &[name, ("value", Value::Bool(i.value != 0.0))]),
            InputKind::Number => b.add("StateMachineNumber", &[name, ("value", Double(i.value as f32))]),
            InputKind::Trigger => b.add("StateMachineTrigger", &[name]),
        };
    }
    for layer in &sm.layers {
        b.add("StateMachineLayer", &[("name", Str(layer.name.clone()))]);
        // States in order (their index is their id in the layer); each followed by its
        // blend children, then its transitions, each followed by its conditions.
        for (si, st) in layer.states.iter().enumerate() {
            let flags = u64::from(st.random);
            match &st.kind {
                StateKind::Entry => b.add("EntryState", &[]),
                StateKind::Any => b.add("AnyState", &[]),
                StateKind::Exit => b.add("ExitState", &[]),
                StateKind::Animation { animation, speed } => {
                    let mut p = vec![("animationId", Uint(*animation as u64))];
                    if *speed != 1.0 {
                        p.push(("speed", Double(*speed)));
                    }
                    if flags != 0 {
                        p.push(("flags", Uint(flags)));
                    }
                    b.add("AnimationState", &p)
                }
                StateKind::Blend1D { input, children } => {
                    // Flag 2 (reset) makes the mix a true weighted average; without it Rive layers
                    // the poses one after another and a 50/50 blend lands at two thirds.
                    let p = vec![("inputId", Uint(*input as u64)), ("flags", Uint(flags | 2))];
                    let at = b.add("BlendState1DInput", &p);
                    for (anim, value) in children {
                        b.add("BlendAnimation1D", &[("animationId", Uint(*anim as u64)), ("value", Double(*value))]);
                    }
                    at
                }
            };
            for t in layer.transitions.iter().filter(|t| t.from == si) {
                let mut p = vec![("stateToId", Uint(t.to as u64))];
                let mut flags = 0u64;
                if t.duration_ms > 0 {
                    p.push(("duration", Uint(t.duration_ms as u64)));
                }
                if let Some(ms) = t.exit_after_ms {
                    flags |= 4; // exit time enabled, in milliseconds
                    p.push(("exitTime", Uint(ms as u64)));
                }
                if flags != 0 {
                    p.push(("flags", Uint(flags)));
                }
                if t.random_weight != 1 {
                    p.push(("randomWeight", Uint(t.random_weight as u64)));
                }
                b.add("StateTransition", &p);
                for c in &t.conditions {
                    let input = ("inputId", Uint(c.input as u64));
                    match sm.inputs.get(c.input).map(|i| i.kind) {
                        // For a bool, op 0 means "is true" and 1 "is false".
                        Some(InputKind::Bool) => b.add("TransitionBoolCondition", &[input, ("opValue", Uint(u64::from(c.value == 0.0)))]),
                        Some(InputKind::Number) => {
                            b.add("TransitionNumberCondition", &[input, ("opValue", Uint(c.op as u64)), ("value", Double(c.value as f32))])
                        }
                        Some(InputKind::Trigger) => b.add("TransitionTriggerCondition", &[input]),
                        None => continue,
                    };
                }
            }
        }
    }
    for l in &sm.listeners {
        let Some((_, _, p)) = placed.iter().find(|(id, _, _)| *id == l.target) else { continue };
        b.add("StateMachineListenerSingle", &[("targetId", Uint(p.shape)), ("listenerTypeValue", Uint(l.event as u64))]);
        for a in &l.actions {
            match a {
                Action::SetBool { input, value } => {
                    b.add("ListenerBoolChange", &[("inputId", Uint(*input as u64)), ("value", Uint(u64::from(*value)))])
                }
                Action::ToggleBool { input } => b.add("ListenerBoolChange", &[("inputId", Uint(*input as u64)), ("value", Uint(2))]),
                Action::SetNumber { input, value } => {
                    b.add("ListenerNumberChange", &[("inputId", Uint(*input as u64)), ("value", Double(*value as f32))])
                }
                Action::FireTrigger { input } => b.add("ListenerTriggerChange", &[("inputId", Uint(*input as u64))]),
                Action::FireEvent { event } => match events.get(*event) {
                    Some(id) => b.add("ListenerFireEvent", &[("eventId", Uint(*id))]),
                    None => continue,
                },
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo() -> Document {
        let mut d = Document::new();
        d.shapes.push(Shape::new(1, "ball", ShapeKind::Ellipse, 100.0, 250.0, 80.0, 80.0, 0xffff6b6b));
        d.animations[0].tracks.push(Track {
            shape_id: 1,
            prop: Prop::X,
            point: None,
            keys: vec![
                Key { frame: 0, value: 100.0, ease: Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 } },
                Key { frame: 60, value: 400.0, ease: Ease::Linear },
            ],
        });
        d.animations[0].tracks.push(Track {
            shape_id: 1,
            prop: Prop::Fill,
            point: None,
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
        d.animations[0].tracks.push(Track { shape_id: 9, prop: Prop::Rotation, point: None, keys: vec![Key { frame: 0, value: 0.0, ease: Ease::Linear }, Key { frame: 60, value: 90.0, ease: Ease::Linear }] });
        let bytes = d.export_riv();
        let file = vectro_riv::read(&bytes, &Schema::bundled()).expect("valid .riv");
        let node = file.objects.iter().position(|o| o.type_name.as_deref() == Some("Node")).expect("a Node");
        let shape = file.objects.iter().find(|o| o.type_name.as_deref() == Some("Shape")).unwrap();
        let parent = shape.properties.iter().find(|p| p.name.as_deref() == Some("parentId")).unwrap();
        // Component index = file position - 1 (the backboard comes first).
        assert_eq!(parent.value, Value::Uint(node as u64 - 1));
    }

    #[test]
    fn point_animation_bends_a_path_and_exports_to_its_vertex() {
        let imp = svg::import(r##"<svg viewBox="0 0 100 100"><path d="M0 0 L100 0 L100 100 Z" fill="#fff"/></svg>"##).unwrap();
        let mut d = Document::new();
        d.shapes = imp.shapes;
        let id = d.shapes[0].id;
        let x0 = d.shapes[0].paths[0].points[1].x as f64;
        let pt = Some(PointRef { path: 0, index: 1 });
        d.animations[0].tracks.push(Track { shape_id: id, prop: Prop::PointX, point: pt, keys: vec![
            Key { frame: 0, value: x0, ease: Ease::Linear }, Key { frame: 60, value: x0 + 20.0, ease: Ease::Linear }] });
        d.animations[0].tracks.push(Track { shape_id: id, prop: Prop::OutAngle, point: pt, keys: vec![
            Key { frame: 0, value: 90.0, ease: Ease::Linear }] });
        let mid = d.evaluate(Some(0), 30.0);
        assert!((mid[0].paths[0].points[1].x as f64 - (x0 + 10.0)).abs() < 1e-3);
        let file = vectro_riv::read(&d.export_riv(), &Schema::bundled()).expect("valid .riv");
        let names: Vec<_> = file.objects.iter().filter_map(|o| o.type_name.clone()).collect();
        assert!(names.iter().any(|n| n == "CubicDetachedVertex"), "an animated handle forces a curve point");
        let keyed: Vec<u64> = file.objects.iter().filter(|o| o.type_name.as_deref() == Some("KeyedProperty"))
            .filter_map(|o| match o.properties[0].value { Value::Uint(k) => Some(k), _ => None }).collect();
        assert!(keyed.contains(&24) && keyed.contains(&86), "keys Vertex.x (24) and outRotation (86)");
    }

    #[test]
    fn state_machine_exports_in_rive_order() {
        let mut d = demo();
        d.animations.push(Animation::new("full", 60, 1, LoopMode::OneShot));
        d.events.push("done".into());
        let mut layer = Layer::new("Fill");
        let bar = layer.blend("Bar", 0, vec![(0, 0.0), (1, 100.0)]);
        let party = layer.animation("Party", 1);
        layer.go(0, bar);
        layer.go(bar, party).on_trigger(2).blend_ms(150);
        layer.go(party, bar).after_ms(800);
        d.state_machines.push(StateMachine {
            name: "Loading".into(),
            inputs: vec![
                Input { name: "progress".into(), kind: InputKind::Number, value: 0.0 },
                Input { name: "hover".into(), kind: InputKind::Bool, value: 0.0 },
                Input { name: "done".into(), kind: InputKind::Trigger, value: 0.0 },
            ],
            layers: vec![layer],
            listeners: vec![Listener { target: 1, event: Pointer::Enter, actions: vec![Action::SetBool { input: 1, value: true }, Action::FireEvent { event: 0 }] }],
        });
        let file = vectro_riv::read(&d.export_riv(), &Schema::bundled()).expect("valid .riv");
        let names: Vec<&str> = file.objects.iter().filter_map(|o| o.type_name.as_deref()).collect();
        let order = ["Event", "StateMachine", "StateMachineNumber", "StateMachineBool", "StateMachineTrigger", "StateMachineLayer",
            "EntryState", "StateTransition", "AnyState", "ExitState", "BlendState1DInput", "BlendAnimation1D", "BlendAnimation1D",
            "StateTransition", "TransitionTriggerCondition", "AnimationState", "StateTransition", "StateMachineListenerSingle",
            "ListenerBoolChange", "ListenerFireEvent"];
        let mut pos = 0;
        for want in order {
            pos = names[pos..].iter().position(|n| *n == want).map(|p| p + pos + 1).unwrap_or_else(|| panic!("{want} missing or out of order"));
        }
        assert_eq!(names.iter().filter(|n| **n == "StateMachine").count(), 1, "no default machine when one is given");
    }

    #[test]
    fn paints_export_and_animate_gradient_stops() {
        let mut d = Document::new();
        d.shapes.push(Shape::new(1, "bar", ShapeKind::Rectangle, 100.0, 50.0, 200.0, 20.0, 0xffffffff)
            .gradient(Gradient::linear(-100.0, 0.0, 100.0, 0.0, &[(0.0, 0xffff8a00), (1.0, 0xffffc837)]))
            .stroke(0xff000000, 2.0)
            .feather(12.0, 0.0, 4.0));
        d.animations[0].tracks.push(Track { shape_id: 1, prop: Prop::FillEnd, point: None, keys: vec![Key { frame: 0, value: 0xffffc837u32 as f64, ease: Ease::Linear }] });
        let file = vectro_riv::read(&d.export_riv(), &Schema::bundled()).expect("valid .riv");
        let names: Vec<&str> = file.objects.iter().filter_map(|o| o.type_name.as_deref()).collect();
        for want in ["LinearGradient", "GradientStop", "Feather", "Stroke", "KeyFrameColor"] {
            assert!(names.contains(&want), "missing {want}");
        }
        let keyed = file.objects.iter().find(|o| o.type_name.as_deref() == Some("KeyedProperty")).unwrap();
        assert_eq!(keyed.properties[0].value, Value::Uint(38), "GradientStop.colorValue, not SolidColor's 37");
        assert_eq!(d.evaluate(Some(0), 0.0)[0].gradient.as_ref().unwrap().stops[1].color, 0xffffc837);
    }

    #[test]
    fn json_round_trip() {
        let d = demo();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<Document>(&json).unwrap(), d);
    }
}
