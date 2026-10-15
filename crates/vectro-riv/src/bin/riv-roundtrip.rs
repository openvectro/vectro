//! Read each .riv file, write it back, and check the bytes are identical.

use std::process::ExitCode;
use vectro_riv::{read, write, Schema};

fn main() -> ExitCode {
    let schema = Schema::bundled();
    let mut failed = false;
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path).expect("readable file");
        match read(&bytes, &schema) {
            Ok(file) => {
                let back = write(&file);
                if back == bytes {
                    println!("identical  {} bytes  {path}", bytes.len());
                } else {
                    let at = back.iter().zip(&bytes).position(|(a, b)| a != b).unwrap_or(back.len().min(bytes.len()));
                    println!("DIFFERS at byte {at} ({} vs {} bytes)  {path}", back.len(), bytes.len());
                    failed = true;
                }
            }
            Err(e) => {
                println!("UNREADABLE {e}  {path}");
                failed = true;
            }
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
