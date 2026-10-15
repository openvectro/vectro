//! vectro export <doc.vectro.json> <out.riv>
//! vectro import-svg <file.svg> [doc.vectro.json]   (prints the document with the SVG added)

use std::process::ExitCode;
use vectro_doc::Document;

fn run(args: &[String]) -> Result<(), String> {
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    match args {
        [cmd, doc, out] if cmd == "export" => {
            let d: Document = serde_json::from_str(&read(doc)?).map_err(|e| format!("{doc}: {e}"))?;
            let bytes = d.export_riv();
            std::fs::write(out, &bytes).map_err(|e| format!("{out}: {e}"))?;
            eprintln!("wrote {out} ({} bytes)", bytes.len());
            Ok(())
        }
        [cmd, svg, rest @ ..] if cmd == "import-svg" && rest.len() <= 1 => {
            let mut d = match rest.first() {
                Some(doc) => serde_json::from_str(&read(doc)?).map_err(|e| format!("{doc}: {e}"))?,
                None => Document::new(),
            };
            d.import_svg(&read(svg)?)?;
            println!("{}", serde_json::to_string_pretty(&d).unwrap());
            Ok(())
        }
        _ => Err("usage: vectro export <doc.vectro.json> <out.riv> | vectro import-svg <file.svg> [doc.vectro.json]".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
