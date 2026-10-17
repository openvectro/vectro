//! Draw-on: a stroked curve whose trim end goes 0 to 1 over one second, so the line draws
//! itself the way Rive's TrimPath does (a mask along the outline, not moving points).
//!
//! cargo run --release -p vectro-doc --example draw_on -- tests/web/draw-on.riv

use vectro_doc::{Animation, Document, Ease, LoopMode, Prop, Shape, ShapeKind, SubPath, Vertex};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "draw-on.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Draw on".into();
    d.artboard.width = 300.0;
    d.artboard.height = 200.0;
    d.artboard.background = 0xff1e1e24;

    // An S curve across the artboard, 20 px in from each side, drawn with a 10 px round stroke.
    let mut line = Shape::new(1, "line", ShapeKind::Path, 150.0, 100.0, 0.0, 0.0, 0x00000000).stroke(0xffffc837, 10.0).trim(0.0, 0.0);
    line.paths = vec![SubPath {
        closed: false,
        points: vec![
            Vertex { x: -130.0, y: 0.0, in_x: -130.0, in_y: 0.0, out_x: -60.0, out_y: -90.0 },
            Vertex { x: 130.0, y: 0.0, in_x: 60.0, in_y: 90.0, out_x: 130.0, out_y: 0.0 },
        ],
    }];
    d.shapes.push(line);

    let mut draw = Animation::new("Draw", 60, 60, LoopMode::OneShot);
    draw.key(1, Prop::TrimEnd, &[(0, 0.0, Ease::Cubic { x1: 0.42, y1: 0.0, x2: 0.58, y2: 1.0 }), (60, 1.0, Ease::Hold)]);
    d.animations = vec![draw];

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    println!("wrote {out} ({} bytes)", bytes.len());
}
