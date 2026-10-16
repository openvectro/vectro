//! A loading bar driven by a number input, with a hover shimmer and a "done" celebration.
//! Everything interactive lives in the file; the app only sets `progress` and fires `done`.
//!
//! cargo run --release -p vectro-doc --example loading_bar -- tests/web/loading-bar.riv

use vectro_doc::{Action, Animation, Document, Ease, Input, InputKind, Layer, Listener, LoopMode, Pointer, Prop, Shape, ShapeKind, StateMachine};

const ORANGE: u32 = 0xfff9a61a;
const GREEN: u32 = 0xff51cf66;
const TRACK: u32 = 0xff2a2a33;
const WHITE: u32 = 0xffffffff;
const LEFT: f32 = 60.0;
const FULL: f32 = 520.0;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "loading-bar.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Loading bar".into();
    d.artboard.width = 640.0;
    d.artboard.height = 200.0;

    let (track, bar, shimmer, tick) = (1, 2, 3, 4);
    d.shapes = vec![
        Shape::new(track, "track", ShapeKind::Rectangle, 320.0, 100.0, FULL, 28.0, TRACK).radius(14.0),
        Shape::new(bar, "bar", ShapeKind::Rectangle, LEFT + 1.0, 100.0, 2.0, 28.0, ORANGE).radius(14.0),
        Shape::new(shimmer, "shimmer", ShapeKind::Ellipse, LEFT, 100.0, 60.0, 28.0, WHITE).opacity(0.0),
        Shape::new(tick, "tick", ShapeKind::Ellipse, 600.0, 100.0, 24.0, 24.0, GREEN).opacity(0.0),
    ];

    let hold = Ease::Hold;
    let spring = Ease::Cubic { x1: 0.34, y1: 1.56, x2: 0.64, y2: 1.0 };
    let smooth = Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };

    // 0 and 1: the two poses the number input blends between.
    let mut empty = Animation::new("empty", 60, 1, LoopMode::OneShot);
    empty.key(bar, Prop::Width, &[(0, 2.0, hold)]).key(bar, Prop::X, &[(0, (LEFT + 1.0) as f64, hold)]);
    empty.key(bar, Prop::Fill, &[(0, ORANGE as f64, hold)]).key(bar, Prop::ScaleY, &[(0, 1.0, hold)]).key(tick, Prop::Opacity, &[(0, 0.0, hold)]);
    let mut full = Animation::new("full", 60, 1, LoopMode::OneShot);
    full.key(bar, Prop::Width, &[(0, FULL as f64, hold)]).key(bar, Prop::X, &[(0, (LEFT + FULL / 2.0) as f64, hold)]);
    full.key(bar, Prop::Fill, &[(0, ORANGE as f64, hold)]).key(bar, Prop::ScaleY, &[(0, 1.0, hold)]).key(tick, Prop::Opacity, &[(0, 0.0, hold)]);
    // 2 and 3: the shimmer, off and sweeping.
    let mut still = Animation::new("still", 60, 1, LoopMode::OneShot);
    still.key(shimmer, Prop::Opacity, &[(0, 0.0, hold)]);
    let mut glow = Animation::new("glow", 60, 72, LoopMode::Loop);
    glow.key(shimmer, Prop::Opacity, &[(0, 0.35, hold)]).key(shimmer, Prop::X, &[(0, LEFT as f64, Ease::Linear), (72, (LEFT + FULL) as f64, hold)]);
    // 4: the celebration.
    let mut party = Animation::new("celebrate", 60, 60, LoopMode::OneShot);
    party.key(bar, Prop::Fill, &[(0, ORANGE as f64, smooth), (12, GREEN as f64, hold)]);
    party.key(bar, Prop::ScaleY, &[(0, 1.0, spring), (8, 1.22, smooth), (24, 1.0, hold)]);
    party.key(tick, Prop::Opacity, &[(0, 0.0, hold), (6, 1.0, hold)]);
    party.key(tick, Prop::ScaleX, &[(0, 0.0, hold), (6, 0.0, spring), (16, 1.3, smooth), (28, 1.0, hold)]);
    party.key(tick, Prop::ScaleY, &[(0, 0.0, hold), (6, 0.0, spring), (16, 1.3, smooth), (28, 1.0, hold)]);
    d.animations = vec![empty, full, still, glow, party];

    d.events = vec!["clicked".into()];
    let (progress, hover, done) = (0, 1, 2);
    let mut fill = Layer::new("Fill");
    let blend = fill.blend("Bar", progress, vec![(0, 0.0), (1, 100.0)]);
    let celebrate = fill.animation("Celebrate", 4);
    fill.go(0, blend);
    fill.go(blend, celebrate).on_trigger(done).blend_ms(100);
    fill.go(celebrate, blend).after_ms(1000).blend_ms(200);

    let mut shine = Layer::new("Shine");
    let off = shine.animation("Still", 2);
    let on = shine.animation("Glow", 3);
    shine.go(0, off);
    shine.go(off, on).when_true(hover).blend_ms(150);
    shine.go(on, off).when_false(hover).blend_ms(250);

    d.state_machines.push(StateMachine {
        name: "Loading".into(),
        inputs: vec![
            Input { name: "progress".into(), kind: InputKind::Number, value: 0.0 },
            Input { name: "hover".into(), kind: InputKind::Bool, value: 0.0 },
            Input { name: "done".into(), kind: InputKind::Trigger, value: 0.0 },
        ],
        layers: vec![fill, shine],
        listeners: vec![
            Listener { target: track, event: Pointer::Enter, actions: vec![Action::SetBool { input: hover, value: true }] },
            Listener { target: track, event: Pointer::Exit, actions: vec![Action::SetBool { input: hover, value: false }] },
            Listener { target: track, event: Pointer::Click, actions: vec![Action::FireTrigger { input: done }, Action::FireEvent { event: 0 }] },
        ],
    });

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
