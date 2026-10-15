# Vectro

Free, open-source interactive vector animation: layers, vectors, timelines and state
machines, for every platform. Vectro exports `.riv` files, so what you make plays on the
existing free runtimes for web, Flutter, iOS and Android.

Not affiliated with Rive. "Rive" is their trademark; Vectro reads and writes the format
they publish.

## Status

Early, but it works end to end. Draw rectangles and ellipses or import SVG paths, key position, size,
rotation, scale, opacity, corner radius and colour on a timeline with easing, play it
back, and export `.riv`. Rive's own web runtime plays the export the same way the editor
shows it (checked frame by frame). State machines in the editor come next; the format
side is already proven (`crates/vectro-riv/examples/button.rs`).

## Run the editor

```
cd editor
npm install
npm run wasm   # builds the Rust core for the browser (needs wasm-pack)
npm run dev    # http://127.0.0.1:5173
```

Keys: V select, R rectangle, E ellipse, Space play, ⌫ delete, ⌘Z / ⇧⌘Z undo / redo.

## Tools

```
cargo run --release --bin vectro -- import-svg logo.svg > logo.vectro.json
cargo run --release --bin vectro -- export logo.vectro.json logo.riv
cargo run --release --bin riv-dump -- path/to/file.riv
cargo run --release --bin riv-roundtrip -- path/to/*.riv
cargo test
```

## Layout

- `crates/vectro-riv` reads and writes `.riv`; `riv-dump`, `riv-roundtrip`
- `crates/vectro-doc` the document, playback maths and `.riv` export
- `crates/vectro-wasm` the core compiled for the browser
- `editor/` the web editor (Svelte), with a preview that uses Rive's own runtime
- `data/rive-keys.json` type and property table from Rive's MIT runtime
- `docs/format.md` what the format holds, confirmed on real files

MIT licence.
