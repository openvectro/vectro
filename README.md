# Vectro

Free, open-source interactive vector animation: layers, vectors, timelines and state
machines, for every platform. Vectro exports `.riv` files, so what you make plays on the
existing free runtimes for web, Flutter, iOS and Android.

Not affiliated with Rive. "Rive" is their trademark; Vectro reads and writes the format
they publish.

## Status

Early. `vectro-riv` reads `.riv` (format 7). Writer and editor come next.

```
cargo run --release --bin riv-dump -- path/to/file.riv
```

## Layout

- `crates/vectro-riv` reader (writer to follow) and `riv-dump`
- `data/rive-keys.json` type and property table from Rive's MIT runtime
- `docs/format.md` what the format holds, confirmed on real files

MIT licence.
