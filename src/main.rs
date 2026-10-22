//! `cortex9-kvm-vmm`: arranca el guest incluido y vuelca su consola serie.

use cortex9_kvm_vmm::guest::GUEST_IMAGE;
use cortex9_kvm_vmm::vmm::{run_guest, GuestExit};
use std::io::Write;
use std::process::ExitCode;

const DEFAULT_MAX_EXITS: u64 = 100_000;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut image = GUEST_IMAGE.to_vec();
    let mut max_exits = DEFAULT_MAX_EXITS;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--image" => {
                let path = args.next().expect("--image necesita una ruta");
                image = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            }
            "--max-exits" => {
                max_exits = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--max-exits necesita un entero");
            }
            "-h" | "--help" => {
                println!("uso: cortex9-kvm-vmm [--image RUTA] [--max-exits N]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("argumento desconocido: {other}");
                return ExitCode::from(2);
            }
        }
    }
    match run_guest(&image, max_exits) {
        Ok(report) => {
            let _ = std::io::stdout().write_all(&report.serial);
            eprintln!(
                "[vmm] salida={:?} salidas_de_vm={}",
                report.exit, report.vm_exits
            );
            match report.exit {
                GuestExit::DebugExit(0) | GuestExit::Hlt => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            }
        }
        Err(e) => {
            eprintln!("[vmm] error: {e}");
            ExitCode::FAILURE
        }
    }
}
