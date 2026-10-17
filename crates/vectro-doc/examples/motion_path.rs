//! A motion path: a small circle follows an open curve from one end to the other in two seconds.
//! The curve is drawn as a thin grey line so the path is visible in the player.
//!
//! cargo run --release -p vectro-doc --example motion_path -- tests/web/motion-path.riv

use vectro_doc::{Animation, Document, Ease, LoopMode, Prop, Shape, ShapeKind, SubPath, Vertex};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "motion-path.riv".into());
    let mut d = Document::new();
    d.artboard.name = "Motion path".into();
    d.artboard.width = 300.0;
    d.artboard.height = 200.0;
    d.artboard.background = 0xff202020;

    // An open S-curve from the bottom left to the top right, points relative to the shape's origin.
    let (track, dot) = (1, 2);
    let mut curve = Shape::new(track, "track", ShapeKind::Path, 150.0, 100.0, 0.0, 0.0, 0x00000000).stroke(0xff6c6c74, 2.0);
    curve.paths = vec![SubPath {
        closed: false,
        points: vec![
            Vertex { x: -110.0, y: 60.0, in_x: -110.0, in_y: 60.0, out_x: -60.0, out_y: 60.0 },
            Vertex { x: 0.0, y: 0.0, in_x: -50.0, in_y: 0.0, out_x: 50.0, out_y: 0.0 },
            Vertex { x: 110.0, y: -60.0, in_x: 60.0, in_y: -60.0, out_x: 110.0, out_y: -60.0 },
        ],
    }];
    d.shapes.push(curve);
    d.shapes.push(Shape::new(dot, "dot", ShapeKind::Ellipse, 0.0, 0.0, 20.0, 20.0, 0xffff3b30).follow(track, true));

    let mut a = Animation::new("Follow", 60, 120, LoopMode::OneShot);
    a.key(dot, Prop::FollowDistance, &[(0, 0.0, Ease::Linear), (120, 1.0, Ease::Hold)]);
    d.animations = vec![a];

    let bytes = d.export_riv();
    std::fs::write(&out, &bytes).expect("write output");
    std::fs::write(out.replace(".riv", ".vectro.json"), serde_json::to_string_pretty(&d).unwrap()).expect("write project");
    // Where the dot should be, from the editor's own maths, for checking the runtime against.
    for f in [0.0, 60.0, 120.0] {
        let s = d.evaluate(Some(0), f).into_iter().find(|s| s.id == dot).unwrap();
        println!("frame {f}: dot at ({:.1}, {:.1}) facing {:.0} degrees", s.x, s.y, s.rotation);
    }
    println!("wrote {out} ({} bytes)", bytes.len());
}
