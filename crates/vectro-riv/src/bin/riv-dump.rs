//! Print every object in a .riv file: `riv-dump <file.riv>`.

use std::io::Write;
use std::process::ExitCode;
use vectro_riv::{read, Schema};

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: riv-dump <file.riv>");
        return ExitCode::FAILURE;
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let file = match read(&bytes, &Schema::bundled()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{path}: format {}.{}, file id {}, {} objects, {} properties in table of contents",
        file.major, file.minor, file.file_id, file.objects.len(), file.toc.len());
    for (i, o) in file.objects.iter().enumerate() {
        let name = o.type_name.as_deref().unwrap_or("?");
        let props: Vec<String> = o.properties.iter()
            .map(|p| format!("{}={}", p.name.as_deref().unwrap_or(&p.key.to_string()), p.value))
            .collect();
        if writeln!(out, "{i:4} {name}({}) {}", o.type_key, props.join(" ")).is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}
