//! Vectro's own logo, animated with Vectro: the tile pops in, the V draws itself the way a pen
//! stroke does (its points travel, not a mask), the anchor and handle appear last.
//!
//! cargo run --release -p vectro-doc --example logo -- brand/vectro-logo.riv

use vectro_doc::{Animation, Document, Ease, Gradient, Key, LoopMode, PointRef, Prop, Shape, ShapeKind, SubPath, Track, Vertex};

const WHITE: u32 = 0xffffffff;
const CLEAR: u32 = 0x00000000;

fn v(x: f32, y: f32) -> Vertex {
    Vertex { x, y, in_x: x, in_y: y, out_x: x, out_y: y }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "vectro-logo.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Vectro logo".into();
    d.artboard.width = 512.0;
    d.artboard.height = 512.0;
    d.artboard.background = 0x00000000;

    // Geometry from brand/build.py, moved so the tile's centre is the origin.
    let o = |p: (f32, f32)| (p.0 - 256.0, p.1 - 256.0);
    let (a, b) = (o((128.0, 170.0)), o((236.0, 366.0)));
    let (c1, c2, c) = (o((272.0, 300.0)), o((296.0, 234.0)), o((334.0, 176.0)));
    let h = o((412.0, 118.0));

    let (tile, vee, handle, anchor, hole, knob) = (1, 2, 3, 4, 5, 6);
    d.shapes.push(Shape::new(tile, "tile", ShapeKind::Rectangle, 256.0, 256.0, 512.0, 512.0, 0xff4f8cff)
        .radius(112.0)
        .gradient(Gradient::linear(-256.0, -256.0, 256.0, 256.0, &[(0.0, 0xff7c6cff), (0.5, 0xff4f8cff), (1.0, 0xff22d3ee)])));
    // The V: three points, the right arm curved. It starts collapsed onto the vertex and unfolds.
    let mut vee_shape = Shape::new(vee, "V", ShapeKind::Path, 256.0, 256.0, 0.0, 0.0, CLEAR).stroke(WHITE, 60.0);
    let mut pb = v(b.0, b.1);
    pb.out_x = c1.0;
    pb.out_y = c1.1;
    let mut pc = v(c.0, c.1);
    pc.in_x = c2.0;
    pc.in_y = c2.1;
    vee_shape.paths = vec![SubPath { closed: false, points: vec![v(a.0, a.1), pb, pc] }];
    d.shapes.push(vee_shape);
    let mut line = Shape::new(handle, "handle", ShapeKind::Path, 256.0, 256.0, 0.0, 0.0, CLEAR).stroke(WHITE, 9.0);
    line.paths = vec![SubPath { closed: false, points: vec![v(c.0, c.1), v(h.0, h.1)] }];
    d.shapes.push(line);
    // The anchor: a white square with a tile-coloured hole, so it reads as a node on the stroke's cap.
    d.shapes.push(Shape::new(anchor, "anchor", ShapeKind::Rectangle, 256.0 + c.0, 256.0 + c.1, 44.0, 44.0, WHITE).radius(6.0));
    d.shapes.push(Shape::new(hole, "anchor hole", ShapeKind::Rectangle, 256.0 + c.0, 256.0 + c.1, 18.0, 18.0, 0xff4f8cff).radius(3.0));
    d.shapes.push(Shape::new(knob, "knob", ShapeKind::Ellipse, 256.0 + h.0, 256.0 + h.1, 35.0, 35.0, CLEAR).stroke(WHITE, 9.0));

    let hold = Ease::Hold;
    let spring = Ease::Cubic { x1: 0.34, y1: 1.56, x2: 0.64, y2: 1.0 };
    let out_e = Ease::Cubic { x1: 0.16, y1: 1.0, x2: 0.3, y2: 1.0 };
    let smooth = Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 };

    let mut intro = Animation::new("Intro", 60, 180, LoopMode::OneShot);
    // 1. The tile pops in.
    intro.key(tile, Prop::ScaleX, &[(0, 0.0, spring), (22, 1.0, hold)]).key(tile, Prop::ScaleY, &[(0, 0.0, spring), (22, 1.0, hold)]);
    intro.key(tile, Prop::Opacity, &[(0, 0.0, out_e), (8, 1.0, hold)]);
    // 2. The V draws: left arm first (point 0 travels from the vertex up to A), then the right arm
    //    (point 2 travels along the curve while both handles grow out of nothing).
    let pt = |path: u32, index: u32| Some(PointRef { path, index });
    let key = |frame: u32, value: f64, ease: Ease| Key { frame, value, ease };
    intro.key(vee, Prop::Opacity, &[(0, 0.0, hold), (14, 0.0, hold), (15, 1.0, hold)]);
    let mut push = |prop: Prop, point: Option<PointRef>, keys: Vec<Key>| intro.tracks.push(Track { shape_id: vee, prop, point, keys });
    push(Prop::PointX, pt(0, 0), vec![key(0, b.0 as f64, hold), key(16, b.0 as f64, out_e), key(40, a.0 as f64, hold)]);
    push(Prop::PointY, pt(0, 0), vec![key(0, b.1 as f64, hold), key(16, b.1 as f64, out_e), key(40, a.1 as f64, hold)]);
    push(Prop::PointX, pt(0, 2), vec![key(0, b.0 as f64, hold), key(34, b.0 as f64, out_e), key(66, c.0 as f64, hold)]);
    push(Prop::PointY, pt(0, 2), vec![key(0, b.1 as f64, hold), key(34, b.1 as f64, out_e), key(66, c.1 as f64, hold)]);
    let len = |p: (f32, f32), q: (f32, f32)| ((p.0 - q.0).hypot(p.1 - q.1)) as f64;
    let ang = |from: (f32, f32), to: (f32, f32)| ((to.1 - from.1).atan2(to.0 - from.0)).to_degrees() as f64;
    push(Prop::OutLength, pt(0, 1), vec![key(0, 0.0, hold), key(34, 0.0, smooth), key(66, len(b, c1), hold)]);
    push(Prop::OutAngle, pt(0, 1), vec![key(0, ang(b, c1), hold)]);
    push(Prop::InLength, pt(0, 2), vec![key(0, 0.0, hold), key(34, 0.0, smooth), key(66, len(c, c2), hold)]);
    push(Prop::InAngle, pt(0, 2), vec![key(0, ang(c, c2), hold)]);
    // 3. The anchor pops, the handle draws out, the knob pops.
    for s in [anchor, hole] {
        intro.key(s, Prop::ScaleX, &[(0, 0.0, hold), (60, 0.0, spring), (76, 1.0, hold)]).key(s, Prop::ScaleY, &[(0, 0.0, hold), (60, 0.0, spring), (76, 1.0, hold)]);
    }
    intro.key(handle, Prop::Opacity, &[(0, 0.0, hold), (69, 0.0, hold), (70, 1.0, hold)]);
    intro.tracks.push(Track { shape_id: handle, prop: Prop::PointX, point: pt(0, 1), keys: vec![key(0, c.0 as f64, hold), key(70, c.0 as f64, out_e), key(88, h.0 as f64, hold)] });
    intro.tracks.push(Track { shape_id: handle, prop: Prop::PointY, point: pt(0, 1), keys: vec![key(0, c.1 as f64, hold), key(70, c.1 as f64, out_e), key(88, h.1 as f64, hold)] });
    intro.key(knob, Prop::ScaleX, &[(0, 0.0, hold), (84, 0.0, spring), (100, 1.0, hold)]).key(knob, Prop::ScaleY, &[(0, 0.0, hold), (84, 0.0, spring), (100, 1.0, hold)]);
    // 4. A small settle: the whole mark breathes once.
    intro.key(tile, Prop::Rotation, &[(0, -6.0, out_e), (22, 0.0, hold)]);
    d.animations = vec![intro];

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
