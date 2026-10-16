//! Whack-a-dot: three dots pop up at random, click one to hit it. The file reports every hit
//! as an event, so the app keeps the score without knowing anything about the animation.
//!
//! cargo run --release -p vectro-doc --example whack -- tests/web/whack.riv

use vectro_doc::{Action, Animation, Document, Ease, Input, InputKind, Layer, Listener, LoopMode, Pointer, Prop, Shape, ShapeKind, StateMachine};

const ORANGE: u32 = 0xfff9a61a;
const HOLE: u32 = 0xff2a2a33;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "whack.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Whack a dot".into();
    d.artboard.width = 640.0;
    d.artboard.height = 400.0;

    let xs = [160.0f32, 320.0, 480.0];
    let (rest_y, up_y) = (250.0f64, 190.0f64);
    for (i, &x) in xs.iter().enumerate() {
        d.shapes.push(Shape::new(i as u32 + 1, &format!("hole {}", i + 1), ShapeKind::Ellipse, x, 250.0, 120.0, 44.0, HOLE));
    }
    for (i, &x) in xs.iter().enumerate() {
        d.shapes.push(Shape::new(i as u32 + 4, &format!("dot {}", i + 1), ShapeKind::Ellipse, x, 250.0, 72.0, 72.0, ORANGE).opacity(0.0));
    }

    let hold = Ease::Hold;
    let spring = Ease::Cubic { x1: 0.34, y1: 1.56, x2: 0.64, y2: 1.0 };
    let out_ease = Ease::Cubic { x1: 0.16, y1: 1.0, x2: 0.3, y2: 1.0 };
    let smooth = Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };

    // Per dot: 3i down (hidden, long so wait states can time off it), 3i+1 up, 3i+2 hit.
    for i in 0..3u32 {
        let dot = i + 4;
        let mut down = Animation::new(&format!("down {}", i + 1), 60, 240, LoopMode::OneShot);
        down.key(dot, Prop::Opacity, &[(0, 0.0, hold)]).key(dot, Prop::ScaleX, &[(0, 0.2, hold)]).key(dot, Prop::ScaleY, &[(0, 0.2, hold)]).key(dot, Prop::Y, &[(0, rest_y, hold)]);
        let mut up = Animation::new(&format!("up {}", i + 1), 60, 18, LoopMode::OneShot);
        up.key(dot, Prop::Opacity, &[(0, 0.0, Ease::Linear), (4, 1.0, hold)]);
        up.key(dot, Prop::ScaleX, &[(0, 0.2, spring), (10, 1.15, smooth), (18, 1.0, hold)]);
        up.key(dot, Prop::ScaleY, &[(0, 0.2, spring), (10, 1.15, smooth), (18, 1.0, hold)]);
        up.key(dot, Prop::Y, &[(0, rest_y, out_ease), (10, up_y, hold)]);
        let mut hit = Animation::new(&format!("hit {}", i + 1), 60, 20, LoopMode::OneShot);
        hit.key(dot, Prop::ScaleX, &[(0, 1.0, out_ease), (4, 1.5, smooth), (20, 0.0, hold)]);
        hit.key(dot, Prop::ScaleY, &[(0, 1.0, out_ease), (4, 0.5, smooth), (20, 0.0, hold)]);
        hit.key(dot, Prop::Opacity, &[(0, 1.0, hold), (8, 1.0, Ease::Linear), (20, 0.0, hold)]);
        hit.key(dot, Prop::Y, &[(0, up_y, out_ease), (4, up_y + 25.0, hold)]);
        d.animations.extend([down, up, hit]);
    }
    d.animations.remove(0); // the default empty animation

    d.events = vec!["hit".into()];
    let mut sm = StateMachine { name: "Game".into(), inputs: Vec::new(), layers: Vec::new(), listeners: Vec::new() };
    for i in 0..3usize {
        sm.inputs.push(Input { name: format!("hit{}", i + 1), kind: InputKind::Trigger, value: 0.0 });
    }
    for i in 0..3usize {
        let (down_a, up_a, hit_a) = (3 * i, 3 * i + 1, 3 * i + 2);
        let mut layer = Layer::new(&format!("Dot {}", i + 1));
        let down = layer.animation("Down", down_a);
        layer.states[down].random = true;
        let waits: Vec<usize> = ["short", "mid", "long"].iter().map(|w| layer.animation(&format!("Wait {w}"), down_a)).collect();
        let up = layer.animation("Up", up_a);
        let hit = layer.animation("Hit", hit_a);
        layer.go(0, down);
        // Down picks one of three waits at random, each pops the dot up after a different delay.
        for (w, ms) in waits.iter().zip([600u32, 1300, 2200]) {
            layer.go(down, *w);
            layer.go(*w, up).after_ms(ms);
        }
        layer.go(up, down).after_ms(1400).blend_ms(150); // missed it
        layer.go(up, hit).on_trigger(i);
        layer.go(hit, down).after_ms(350);
        sm.layers.push(layer);
        sm.listeners.push(Listener {
            target: i as u32 + 4,
            event: Pointer::Down,
            actions: vec![Action::FireTrigger { input: i }, Action::FireEvent { event: 0 }],
        });
    }
    d.state_machines.push(sm);

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
