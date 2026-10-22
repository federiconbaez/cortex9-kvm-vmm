//! Ensambla el guest (`guest/guest.S`) a una imagen plana con binutils.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn run(cmd: &mut Command) {
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("no se pudo ejecutar {cmd:?}: {e} (¿falta binutils?)"));
    assert!(status.success(), "{cmd:?} terminó con {status}");
}

fn main() {
    println!("cargo:rerun-if-changed=guest/guest.S");
    println!("cargo:rerun-if-changed=guest/guest.ld");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let obj = out.join("guest.o");
    let elf = out.join("guest.elf");
    let bin = out.join("guest.bin");
    let asm = env::var("CORTEX9_AS").unwrap_or_else(|_| "as".into());
    let ld = env::var("CORTEX9_LD").unwrap_or_else(|_| "ld".into());
    let objcopy = env::var("CORTEX9_OBJCOPY").unwrap_or_else(|_| "objcopy".into());
    run(Command::new(&asm)
        .args(["--64", "-o"])
        .arg(&obj)
        .arg("guest/guest.S"));
    run(Command::new(&ld)
        .args([
            "-m",
            "elf_x86_64",
            "-nostdlib",
            "-static",
            "-T",
            "guest/guest.ld",
            "-o",
        ])
        .arg(&elf)
        .arg(&obj));
    run(Command::new(&objcopy)
        .args(["-O", "binary"])
        .arg(&elf)
        .arg(&bin));
}
