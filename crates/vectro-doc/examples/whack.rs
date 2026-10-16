//! Whack-a-mole: three moles pop up at random, click one to bop it. Every hit is reported as an
//! event, so the app keeps score without knowing anything about the animation.
//!
//! cargo run --release -p vectro-doc --example whack -- tests/web/whack.riv

use vectro_doc::{Action, Animation, Document, Ease, Gradient, Input, InputKind, Layer, Listener, LoopMode, Pointer, Prop, Shape, ShapeKind, StateMachine};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "whack.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Whack a mole".into();
    d.artboard.width = 640.0;
    d.artboard.height = 400.0;
    d.artboard.background = 0xff0d0d12;

    let mut next = 0u32;
    let mut id = || {
        next += 1;
        next
    };
    d.shapes.push(Shape::new(id(), "sky", ShapeKind::Rectangle, 320.0, 200.0, 640.0, 400.0, 0xff14141c)
        .gradient(Gradient::linear(0.0, -200.0, 0.0, 200.0, &[(0.0, 0xff1c1c27), (1.0, 0xff0b0b10)])));
    d.shapes.push(Shape::new(id(), "ground", ShapeKind::Rectangle, 320.0, 340.0, 640.0, 120.0, 0xff17171f)
        .gradient(Gradient::linear(0.0, -60.0, 0.0, 60.0, &[(0.0, 0xff1f1f2a), (1.0, 0xff121219)])));

    let xs = [160.0f32, 320.0, 480.0];
    let hole_y = 276.0f32;
    let hold = Ease::Hold;
    let spring = Ease::Cubic { x1: 0.34, y1: 1.56, x2: 0.64, y2: 1.0 };
    let out_ease = Ease::Cubic { x1: 0.16, y1: 1.0, x2: 0.3, y2: 1.0 };
    let smooth = Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };
    let in_ease = Ease::Cubic { x1: 0.55, y1: 0.0, x2: 1.0, y2: 0.45 };

    let mut moles = Vec::new(); // (group, body, pupils, particles)
    for (i, &x) in xs.iter().enumerate() {
        let n = i + 1;
        d.shapes.push(Shape::new(id(), &format!("rim {n}"), ShapeKind::Ellipse, x, hole_y + 2.0, 140.0, 54.0, 0xff2b2b38));
        d.shapes.push(Shape::new(id(), &format!("hole {n}"), ShapeKind::Ellipse, x, hole_y, 122.0, 42.0, 0xff07070a).inner_shadow(10.0, 0.0, 4.0));
    }
    for (i, &x) in xs.iter().enumerate() {
        let n = i + 1;
        let g = id();
        let mut group = Shape::new(g, &format!("mole {n}"), ShapeKind::Group, x, hole_y + 40.0, 0.0, 0.0, 0);
        group.opacity = 0.0;
        d.shapes.push(group);
        let part = |d: &mut Document, s: Shape| d.shapes.push(s.in_group(g));
        part(&mut d, Shape::new(id(), "shadow", ShapeKind::Ellipse, 0.0, 2.0, 96.0, 30.0, 0xff000000).opacity(0.55).feather(14.0, 0.0, 0.0));
        let body = id();
        part(&mut d, Shape::new(body, "body", ShapeKind::Ellipse, 0.0, -46.0, 84.0, 84.0, 0xffff9a2e)
            .gradient(Gradient::radial(-14.0, -18.0, 62.0, &[(0.0, 0xffffd37a), (1.0, 0xffff7a1a)])));
        part(&mut d, Shape::new(id(), "belly", ShapeKind::Ellipse, 0.0, -30.0, 46.0, 34.0, 0xffffe3b0).opacity(0.9));
        for (sx, name) in [(-1.0f32, "left"), (1.0, "right")] {
            part(&mut d, Shape::new(id(), &format!("eye {name}"), ShapeKind::Ellipse, sx * 16.0, -60.0, 15.0, 17.0, 0xffffffff));
        }
        let mut pupils = Vec::new();
        for sx in [-1.0f32, 1.0] {
            let p = id();
            pupils.push(p);
            part(&mut d, Shape::new(p, "pupil", ShapeKind::Ellipse, sx * 16.0 + 2.0, -58.0, 7.0, 8.0, 0xff1a1a1f));
        }
        for sx in [-1.0f32, 1.0] {
            part(&mut d, Shape::new(id(), "cheek", ShapeKind::Ellipse, sx * 27.0, -44.0, 13.0, 9.0, 0xffff6b6b).opacity(0.55));
        }
        part(&mut d, Shape::new(id(), "mouth", ShapeKind::Ellipse, 0.0, -40.0, 11.0, 7.0, 0xff7a2e12));
        let mut particles = Vec::new();
        for _ in 0..5 {
            let p = id();
            particles.push(p);
            part(&mut d, Shape::new(p, "spark", ShapeKind::Ellipse, 0.0, -50.0, 10.0, 10.0, 0xffffc837).opacity(0.0));
        }
        moles.push((g, body, pupils, particles));
    }

    // Per mole: 3i down (long, so wait states can time off it), 3i+1 up, 3i+2 hit.
    let fly = [(-46.0, -96.0), (46.0, -96.0), (-58.0, -46.0), (58.0, -46.0), (0.0, -118.0)];
    for (i, (g, _, pupils, particles)) in moles.iter().enumerate() {
        let g = *g;
        let mut down = Animation::new(&format!("down {}", i + 1), 60, 240, LoopMode::OneShot);
        down.key(g, Prop::Y, &[(0, (hole_y + 40.0) as f64, hold)]).key(g, Prop::Opacity, &[(0, 0.0, hold)]);
        down.key(g, Prop::ScaleX, &[(0, 0.6, hold)]).key(g, Prop::ScaleY, &[(0, 0.6, hold)]);
        for &p in pupils {
            down.key(p, Prop::ScaleX, &[(0, 1.0, hold)]).key(p, Prop::ScaleY, &[(0, 1.0, hold)]);
        }
        for &p in particles {
            down.key(p, Prop::Opacity, &[(0, 0.0, hold)]).key(p, Prop::X, &[(0, 0.0, hold)]).key(p, Prop::Y, &[(0, -50.0, hold)]);
        }
        let mut up = Animation::new(&format!("up {}", i + 1), 60, 20, LoopMode::OneShot);
        up.key(g, Prop::Opacity, &[(0, 0.0, Ease::Linear), (3, 1.0, hold)]);
        up.key(g, Prop::Y, &[(0, (hole_y + 40.0) as f64, out_ease), (12, (hole_y - 8.0) as f64, hold)]);
        up.key(g, Prop::ScaleX, &[(0, 0.6, spring), (12, 1.08, smooth), (20, 1.0, hold)]);
        up.key(g, Prop::ScaleY, &[(0, 0.6, spring), (12, 1.08, smooth), (20, 1.0, hold)]);
        let mut hit = Animation::new(&format!("hit {}", i + 1), 60, 22, LoopMode::OneShot);
        hit.key(g, Prop::ScaleX, &[(0, 1.0, out_ease), (4, 1.4, smooth), (22, 0.5, hold)]);
        hit.key(g, Prop::ScaleY, &[(0, 1.0, out_ease), (4, 0.5, smooth), (22, 0.3, hold)]);
        hit.key(g, Prop::Y, &[(0, (hole_y - 8.0) as f64, out_ease), (4, (hole_y + 14.0) as f64, in_ease), (22, (hole_y + 40.0) as f64, hold)]);
        hit.key(g, Prop::Opacity, &[(0, 1.0, hold), (10, 1.0, Ease::Linear), (22, 0.0, hold)]);
        for &p in pupils {
            hit.key(p, Prop::ScaleX, &[(0, 1.0, out_ease), (3, 1.7, hold)]).key(p, Prop::ScaleY, &[(0, 1.0, out_ease), (3, 1.7, hold)]);
        }
        for (&p, &(fx, fy)) in particles.iter().zip(fly.iter()) {
            hit.key(p, Prop::Opacity, &[(0, 1.0, hold), (8, 1.0, Ease::Linear), (20, 0.0, hold)]);
            hit.key(p, Prop::X, &[(0, 0.0, out_ease), (18, fx, hold)]).key(p, Prop::Y, &[(0, -50.0, out_ease), (18, fy, hold)]);
        }
        d.animations.extend([down, up, hit]);
    }
    d.animations.remove(0); // the default empty animation

    d.events = vec!["hit".into()];
    let mut sm = StateMachine { name: "Game".into(), inputs: Vec::new(), layers: Vec::new(), listeners: Vec::new() };
    for i in 0..3usize {
        sm.inputs.push(Input { name: format!("hit{}", i + 1), kind: InputKind::Trigger, value: 0.0 });
    }
    for (i, (_, body, _, _)) in moles.iter().enumerate() {
        let (down_a, up_a, hit_a) = (3 * i, 3 * i + 1, 3 * i + 2);
        let mut layer = Layer::new(&format!("Mole {}", i + 1));
        let down = layer.animation("Down", down_a);
        layer.states[down].random = true;
        let waits: Vec<usize> = ["short", "mid", "long"].iter().map(|w| layer.animation(&format!("Wait {w}"), down_a)).collect();
        let up = layer.animation("Up", up_a);
        let hit = layer.animation("Hit", hit_a);
        layer.go(0, down);
        // Down picks one of three waits at random; each pops the mole up after a different delay.
        for (w, ms) in waits.iter().zip([500u32, 1200, 2100]) {
            layer.go(down, *w);
            layer.go(*w, up).after_ms(ms);
        }
        layer.go(up, down).after_ms(1500).blend_ms(180); // missed it
        layer.go(up, hit).on_trigger(i);
        layer.go(hit, down).after_ms(380);
        sm.layers.push(layer);
        sm.listeners.push(Listener { target: *body, event: Pointer::Down, actions: vec![Action::FireTrigger { input: i }, Action::FireEvent { event: 0 }] });
    }
    d.state_machines.push(sm);

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
