//! A button built from scratch: hover and press states driven by listeners
//! inside the file, so the app needs no code. Writes `button.riv`.
//!
//! cargo run --release --example button -- out/button.riv

use vectro_riv::{write, Builder, Schema, Value};

use Value::{Color, Double, String as Str, Uint};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "button.riv".into());
    let schema = Schema::bundled();
    let mut b = Builder::new(&schema);

    // Artboard components are referenced by index: 0 artboard, 1 shape, 2 rect, 3 fill, 4 colour.
    b.add("Backboard", &[]);
    b.add("Artboard", &[("name", Str("Button".into())), ("width", Double(300.0)), ("height", Double(200.0))]);
    b.add("Shape", &[("name", Str("button".into())), ("parentId", Uint(0)), ("x", Double(150.0)), ("y", Double(100.0))]);
    let r = 24.0;
    b.add("Rectangle", &[
        ("parentId", Uint(1)), ("width", Double(200.0)), ("height", Double(72.0)),
        ("cornerRadiusTL", Double(r)), ("cornerRadiusTR", Double(r)), ("cornerRadiusBL", Double(r)), ("cornerRadiusBR", Double(r)),
    ]);
    b.add("Fill", &[("parentId", Uint(1))]);
    b.add("SolidColor", &[("parentId", Uint(3)), ("colorValue", Color(0xff3b5bdb))]);

    // Animations 0 Idle, 1 Hover, 2 Pressed: one pose each; the transitions blend between them.
    for (name, scale, colour) in [("Idle", 1.0, 0xff3b5bdbu32), ("Hover", 1.08, 0xff5c7cfa), ("Pressed", 0.94, 0xff2b44a8)] {
        b.add("LinearAnimation", &[("name", Str(name.into())), ("fps", Uint(60)), ("duration", Uint(1))]);
        b.add("KeyedObject", &[("objectId", Uint(1))]);
        for prop in ["scaleX", "scaleY"] {
            b.add("KeyedProperty", &[("propertyKey", Uint(schema.property_key("Shape", prop).unwrap()))]);
            b.add("KeyFrameDouble", &[("frame", Uint(0)), ("value", Double(scale))]);
        }
        b.add("KeyedObject", &[("objectId", Uint(4))]);
        b.add("KeyedProperty", &[("propertyKey", Uint(schema.property_key("SolidColor", "colorValue").unwrap()))]);
        b.add("KeyFrameColor", &[("frame", Uint(0)), ("value", Color(colour))]);
    }

    b.add("StateMachine", &[("name", Str("Button".into()))]);
    b.add("StateMachineBool", &[("name", Str("hover".into()))]); // input 0
    b.add("StateMachineBool", &[("name", Str("pressed".into()))]); // input 1
    b.add("StateMachineLayer", &[("name", Str("Layer 1".into()))]);
    // States by index: 0 entry, 1 any, 2 exit, 3 Idle, 4 Hover, 5 Pressed. Transitions follow their source.
    let to = |b: &mut Builder, state: u64| _ = b.add("StateTransition", &[("stateToId", Uint(state)), ("duration", Uint(150))]);
    let when = |b: &mut Builder, input: u64, is_true: bool| {
        _ = b.add("TransitionBoolCondition", &[("inputId", Uint(input)), ("opValue", Uint(if is_true { 0 } else { 1 }))])
    };
    b.add("EntryState", &[]);
    to(&mut b, 3);
    b.add("AnyState", &[]);
    b.add("ExitState", &[]);
    b.add("AnimationState", &[("animationId", Uint(0))]); // Idle
    to(&mut b, 4);
    when(&mut b, 0, true);
    b.add("AnimationState", &[("animationId", Uint(1))]); // Hover
    to(&mut b, 3);
    when(&mut b, 0, false);
    to(&mut b, 5);
    when(&mut b, 1, true);
    b.add("AnimationState", &[("animationId", Uint(2))]); // Pressed
    to(&mut b, 4);
    when(&mut b, 1, false);
    to(&mut b, 3);
    when(&mut b, 0, false);

    // Listeners on the shape: pointer enter, exit, down, up set the inputs.
    for (event, changes) in [(0u64, vec![(0u64, 1u64)]), (1, vec![(0, 0), (1, 0)]), (2, vec![(1, 1)]), (3, vec![(1, 0)])] {
        b.add("StateMachineListenerSingle", &[("targetId", Uint(1)), ("listenerTypeValue", Uint(event))]);
        for (input, value) in changes {
            b.add("ListenerBoolChange", &[("inputId", Uint(input)), ("value", Uint(value))]);
        }
    }

    let bytes = write(&b.finish());
    std::fs::write(&out, &bytes).expect("write output");
    println!("wrote {out} ({} bytes)", bytes.len());
}
