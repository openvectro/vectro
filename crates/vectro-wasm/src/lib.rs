//! The editor keeps its document as JSON; Rust does the maths and the file format.

use vectro_doc::{cubic_ease, Document};
use wasm_bindgen::prelude::*;

fn parse(doc: &str) -> Result<Document, JsError> {
    serde_json::from_str(doc).map_err(|e| JsError::new(&format!("bad document: {e}")))
}

#[wasm_bindgen(js_name = newDocument)]
pub fn new_document() -> String {
    serde_json::to_string(&Document::new()).unwrap()
}

/// Shapes as they look at `frame` of animation `anim` (-1 for design values), as JSON.
#[wasm_bindgen]
pub fn evaluate(doc: &str, anim: i32, frame: f64) -> Result<String, JsError> {
    let d = parse(doc)?;
    let anim = usize::try_from(anim).ok();
    Ok(serde_json::to_string(&d.evaluate(anim, frame)).unwrap())
}

/// The frame shown `seconds` into animation `anim`, honouring its loop mode.
#[wasm_bindgen(js_name = frameAt)]
pub fn frame_at(doc: &str, anim: usize, seconds: f64) -> Result<f64, JsError> {
    let d = parse(doc)?;
    let a = d.animations.get(anim).ok_or_else(|| JsError::new("no such animation"))?;
    Ok(a.frame_at(seconds))
}

#[wasm_bindgen(js_name = exportRiv)]
pub fn export_riv(doc: &str) -> Result<Vec<u8>, JsError> {
    Ok(parse(doc)?.export_riv())
}

/// One point on an easing curve, for drawing it.
#[wasm_bindgen(js_name = easeAt)]
pub fn ease_at(x1: f32, y1: f32, x2: f32, y2: f32, progress: f64) -> f64 {
    cubic_ease(x1, y1, x2, y2, progress)
}
