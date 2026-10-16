//! A loading bar driven by a number input: a card with a soft shadow, a glowing gradient bar,
//! a hover highlight and a "done" celebration. The app only sets `progress` and fires `done`.
//!
//! cargo run --release -p vectro-doc --example loading_bar -- tests/web/loading-bar.riv

use vectro_doc::{Action, Animation, Document, Ease, Gradient, Input, InputKind, Layer, Listener, LoopMode, Pointer, Prop, Shape, ShapeKind, StateMachine, SubPath, Vertex};

const LEFT: f32 = 80.0;
const FULL: f32 = 480.0;
const Y: f32 = 118.0;
const ORANGE_A: u32 = 0xffff8a00;
const ORANGE_B: u32 = 0xffffc837;
const GREEN_A: u32 = 0xff34d399;
const GREEN_B: u32 = 0xffa7f3d0;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "loading-bar.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Loading bar".into();
    d.artboard.width = 640.0;
    d.artboard.height = 220.0;
    d.artboard.background = 0xff0d0d12;

    let (shadow, card, track, glow, bar, gloss, tick_glow, tick, mark) = (1, 2, 3, 4, 5, 6, 7, 8, 9);
    let check = SubPath {
        closed: false,
        points: [(-6.0, 0.5), (-1.5, 5.0), (7.0, -5.0)].iter().map(|&(x, y)| Vertex { x, y, in_x: x, in_y: y, out_x: x, out_y: y }).collect(),
    };
    let mut mark_shape = Shape::new(mark, "check", ShapeKind::Path, 586.0, Y, 14.0, 10.0, 0x00000000).stroke(0xffffffff, 3.2).opacity(0.0);
    mark_shape.paths = vec![check];
    d.shapes = vec![
        Shape::new(shadow, "card shadow", ShapeKind::Rectangle, 320.0, 118.0, 560.0, 124.0, 0xff000000).radius(26.0).opacity(0.6).feather(30.0, 0.0, 14.0),
        Shape::new(card, "card", ShapeKind::Rectangle, 320.0, 110.0, 560.0, 124.0, 0xff1c1c24)
            .radius(26.0)
            .gradient(Gradient::linear(0.0, -62.0, 0.0, 62.0, &[(0.0, 0xff25252f), (1.0, 0xff15151b)]))
            .stroke(0xff32323f, 1.5),
        Shape::new(track, "track", ShapeKind::Rectangle, 320.0, Y, FULL, 18.0, 0xff0a0a0f).radius(9.0).inner_shadow(8.0, 0.0, 3.0),
        Shape::new(glow, "glow", ShapeKind::Rectangle, LEFT + 2.0, Y + 2.0, 4.0, 30.0, ORANGE_A).radius(15.0).opacity(0.55).feather(22.0, 0.0, 6.0),
        Shape::new(bar, "bar", ShapeKind::Rectangle, LEFT + 2.0, Y, 4.0, 18.0, ORANGE_A)
            .radius(9.0)
            .gradient(Gradient::linear(-240.0, 0.0, 240.0, 0.0, &[(0.0, ORANGE_A), (1.0, ORANGE_B)])),
        Shape::new(gloss, "gloss", ShapeKind::Rectangle, LEFT + 2.0, Y - 5.0, 1.0, 5.0, 0xffffffff).radius(2.5).opacity(0.22),
        Shape::new(tick_glow, "tick glow", ShapeKind::Ellipse, 586.0, Y, 40.0, 40.0, GREEN_A).opacity(0.0).feather(16.0, 0.0, 0.0),
        Shape::new(tick, "tick", ShapeKind::Ellipse, 586.0, Y, 30.0, 30.0, GREEN_A)
            .opacity(0.0)
            .gradient(Gradient::radial(-5.0, -6.0, 20.0, &[(0.0, 0xff6ee7b7), (1.0, 0xff10b981)])),
        mark_shape,
    ];

    let hold = Ease::Hold;
    let spring = Ease::Cubic { x1: 0.34, y1: 1.56, x2: 0.64, y2: 1.0 };
    let smooth = Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };

    // 0 and 1: the poses the number blends between. Both key the same properties.
    let pose = |name: &str, w: f32| {
        let mut a = Animation::new(name, 60, 1, LoopMode::OneShot);
        let x = (LEFT + w / 2.0) as f64;
        for s in [bar, glow] {
            a.key(s, Prop::Width, &[(0, w as f64, hold)]).key(s, Prop::X, &[(0, x, hold)]);
        }
        a.key(gloss, Prop::Width, &[(0, (w - 8.0).max(1.0) as f64, hold)]).key(gloss, Prop::X, &[(0, x, hold)]);
        a.key(bar, Prop::Fill, &[(0, ORANGE_A as f64, hold)]).key(bar, Prop::FillEnd, &[(0, ORANGE_B as f64, hold)]);
        a.key(glow, Prop::Fill, &[(0, ORANGE_A as f64, hold)]).key(bar, Prop::ScaleY, &[(0, 1.0, hold)]);
        for s in [tick_glow, tick, mark] {
            a.key(s, Prop::Opacity, &[(0, 0.0, hold)]).key(s, Prop::ScaleX, &[(0, 0.0, hold)]).key(s, Prop::ScaleY, &[(0, 0.0, hold)]);
        }
        a
    };
    // 2 and 3: the hover highlight, off and on.
    let shine = |name: &str, glow_o: f64, gloss_o: f64| {
        let mut a = Animation::new(name, 60, 1, LoopMode::OneShot);
        a.key(glow, Prop::Opacity, &[(0, glow_o, hold)]).key(gloss, Prop::Opacity, &[(0, gloss_o, hold)]);
        a
    };
    // 4: the celebration.
    let mut party = Animation::new("celebrate", 60, 60, LoopMode::OneShot);
    party.key(bar, Prop::Fill, &[(0, ORANGE_A as f64, smooth), (12, GREEN_A as f64, hold)]);
    party.key(bar, Prop::FillEnd, &[(0, ORANGE_B as f64, smooth), (12, GREEN_B as f64, hold)]);
    party.key(glow, Prop::Fill, &[(0, ORANGE_A as f64, smooth), (12, GREEN_A as f64, hold)]);
    party.key(bar, Prop::ScaleY, &[(0, 1.0, spring), (8, 1.28, smooth), (26, 1.0, hold)]);
    for s in [tick_glow, tick, mark] {
        party.key(s, Prop::Opacity, &[(0, 0.0, hold), (5, 0.0, Ease::Linear), (9, if s == tick_glow { 0.7 } else { 1.0 }, hold)]);
        party.key(s, Prop::ScaleX, &[(0, 0.0, hold), (5, 0.0, spring), (17, 1.3, smooth), (30, 1.0, hold)]);
        party.key(s, Prop::ScaleY, &[(0, 0.0, hold), (5, 0.0, spring), (17, 1.3, smooth), (30, 1.0, hold)]);
    }
    d.animations = vec![pose("empty", 4.0), pose("full", FULL), shine("still", 0.55, 0.22), shine("hot", 0.95, 0.42), party];

    d.events = vec!["clicked".into()];
    let (progress, hover, done) = (0, 1, 2);
    let mut fill = Layer::new("Fill");
    let blend = fill.blend("Bar", progress, vec![(0, 0.0), (1, 100.0)]);
    let celebrate = fill.animation("Celebrate", 4);
    fill.go(0, blend);
    fill.go(blend, celebrate).on_trigger(done).blend_ms(100);
    fill.go(celebrate, blend).after_ms(1100).blend_ms(250);
    let mut shine_layer = Layer::new("Shine");
    let off = shine_layer.animation("Still", 2);
    let on = shine_layer.animation("Hot", 3);
    shine_layer.go(0, off);
    shine_layer.go(off, on).when_true(hover).blend_ms(180);
    shine_layer.go(on, off).when_false(hover).blend_ms(300);

    d.state_machines.push(StateMachine {
        name: "Loading".into(),
        inputs: vec![
            Input { name: "progress".into(), kind: InputKind::Number, value: 0.0 },
            Input { name: "hover".into(), kind: InputKind::Bool, value: 0.0 },
            Input { name: "done".into(), kind: InputKind::Trigger, value: 0.0 },
        ],
        layers: vec![fill, shine_layer],
        listeners: vec![
            Listener { target: card, event: Pointer::Enter, actions: vec![Action::SetBool { input: hover, value: true }] },
            Listener { target: card, event: Pointer::Exit, actions: vec![Action::SetBool { input: hover, value: false }] },
            Listener { target: card, event: Pointer::Click, actions: vec![Action::FireTrigger { input: done }, Action::FireEvent { event: 0 }] },
        ],
    });

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
